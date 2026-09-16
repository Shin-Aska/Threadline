mod facets;
mod hashtags;
use crate::{error::AppError, models::*, providers::SocialProvider};
use async_trait::async_trait;
use serde::Deserialize;
pub struct BlueskyProvider {
    pub search_session: tokio::sync::Mutex<Option<hashtags::SearchSession>>,
    pub capabilities: PlatformCapabilities,
    pub client: reqwest::Client,
    pub service_url: String,
    pub identifier: String,
    pub app_password: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct SessionResponse {
    access_jwt: String,
    did: String,
    handle: String,
}
#[derive(Deserialize)]
struct RecordResponse {
    uri: String,
    cid: String,
}

impl BlueskyProvider {
    async fn session(&self) -> Result<SessionResponse, AppError> {
        self.request(
            self.client.post(format!(
                "{}/xrpc/com.atproto.server.createSession",
                self.service_url.trim_end_matches('/')
            )),
            serde_json::json!({"identifier": self.identifier, "password": self.app_password}),
        )
        .await
    }

    pub async fn account(&self) -> Result<(String, String), AppError> {
        let session = self.session().await?;
        Ok((session.did, session.handle))
    }

    async fn request<T: serde::de::DeserializeOwned>(
        &self,
        request: reqwest::RequestBuilder,
        body: serde_json::Value,
    ) -> Result<T, AppError> {
        let response = request
            .timeout(std::time::Duration::from_secs(60))
            .json(&body)
            .send()
            .await
            .map_err(|error| AppError::Provider(format!("Bluesky request failed: {error}")))?;
        let status = response.status();
        let body = response.text().await.map_err(|error| {
            AppError::Provider(format!("Bluesky response could not be read: {error}"))
        })?;
        if !status.is_success() {
            return Err(AppError::Provider(format!(
                "Bluesky returned {status}: {}",
                crate::providers::safe_error_body(&body)
            )));
        }
        serde_json::from_str(&body)
            .map_err(|error| AppError::Provider(format!("Invalid Bluesky response: {error}")))
    }

    async fn create_record(
        &self,
        post: PreparedPost,
        parent: Option<&PublishedPost>,
    ) -> Result<PublishedPost, AppError> {
        let session = self.session().await?;
        let mut record = serde_json::json!({
            "$type": "app.bsky.feed.post",
            "text": post.text,
            "createdAt": crate::providers::now_iso8601(),
        });
        let facets = facets::hashtags(&post.text)?;
        if !facets.is_empty() {
            record["facets"] = serde_json::to_value(facets)
                .map_err(|error| AppError::Provider(format!("Invalid hashtag facets: {error}")))?;
        }
        if !post.media.is_empty() {
            let mut images = Vec::with_capacity(post.media.len());
            for image in &post.media {
                let response = self
                    .client
                    .post(format!(
                        "{}/xrpc/com.atproto.repo.uploadBlob",
                        self.service_url.trim_end_matches('/')
                    ))
                    .bearer_auth(&session.access_jwt)
                    .header(reqwest::header::CONTENT_TYPE, &image.mime_type)
                    .timeout(std::time::Duration::from_secs(60))
                    .body(image.data.to_vec())
                    .send()
                    .await
                    .map_err(|error| {
                        AppError::Provider(format!("Bluesky image upload failed: {error}"))
                    })?;
                if !response.status().is_success() {
                    return Err(AppError::Provider(format!(
                        "Bluesky image upload returned {}",
                        response.status()
                    )));
                }
                #[derive(Deserialize)]
                struct UploadResponse {
                    blob: serde_json::Value,
                }
                let uploaded: UploadResponse = response.json().await.map_err(|_| {
                    AppError::Provider("Invalid Bluesky image upload response".into())
                })?;
                images.push(serde_json::json!({"alt": image.alt_text, "image": uploaded.blob}));
            }
            record["embed"] =
                serde_json::json!({"$type": "app.bsky.embed.images", "images": images});
        }
        if let Some(parent) = parent {
            let cid = parent.remote_cid.as_ref().ok_or_else(|| {
                AppError::Provider("Bluesky reply parent is missing its CID".into())
            })?;
            let root_uri = parent.root_id.as_ref().unwrap_or(&parent.remote_id);
            let root_cid = parent.root_cid.as_ref().unwrap_or(cid);
            record["reply"] = serde_json::json!({
                "root": {"uri": root_uri, "cid": root_cid},
                "parent": {"uri": parent.remote_id, "cid": cid}
            });
        }
        let response: RecordResponse = self
            .request(
                self.client
                    .post(format!(
                        "{}/xrpc/com.atproto.repo.createRecord",
                        self.service_url.trim_end_matches('/')
                    ))
                    .bearer_auth(session.access_jwt),
                serde_json::json!({
                    "repo": session.did,
                    "collection": "app.bsky.feed.post",
                    "record": record
                }),
            )
            .await?;
        Ok(PublishedPost {
            root_id: parent
                .and_then(|post| post.root_id.clone())
                .or_else(|| parent.map(|post| post.remote_id.clone())),
            root_cid: parent
                .and_then(|post| post.root_cid.clone())
                .or_else(|| parent.and_then(|post| post.remote_cid.clone())),
            remote_id: response.uri,
            remote_cid: Some(response.cid),
        })
    }
}
#[async_trait]
impl SocialProvider for BlueskyProvider {
    async fn timeline(
        &self,
        account_id: &str,
        account_handle: &str,
        cursor: Option<&str>,
    ) -> Result<serde_json::Value, AppError> {
        let session = self.session().await?;
        let mut request = self
            .client
            .get(format!(
                "{}/xrpc/app.bsky.feed.getTimeline",
                self.service_url.trim_end_matches('/')
            ))
            .bearer_auth(session.access_jwt)
            .query(&[("limit", "50")]);
        if let Some(cursor) = cursor {
            request = request.query(&[("cursor", cursor)]);
        }
        let native: serde_json::Value = request
            .send()
            .await
            .map_err(|error| AppError::Provider(format!("Bluesky timeline failed: {error}")))?
            .error_for_status()
            .map_err(|error| AppError::Provider(format!("Bluesky timeline failed: {error}")))?
            .json()
            .await
            .map_err(|error| AppError::Provider(format!("Invalid Bluesky timeline: {error}")))?;
        let posts = native["feed"].as_array().into_iter().flatten().filter_map(|item| { let post=&item["post"]; let uri=post["uri"].as_str()?; let author=&post["author"]; let record=&post["record"]; let handle=author["handle"].as_str().unwrap_or(""); let rkey=uri.rsplit('/').next().unwrap_or(""); let media=item["post"]["embed"]["images"].as_array().map(|images| images.iter().map(|image| serde_json::json!({"url":image["fullsize"],"alt":image["alt"].as_str().unwrap_or(""),"type":"image"})).collect::<Vec<_>>()).unwrap_or_default(); Some(serde_json::json!({"canonicalKey":format!("BLUESKY:{uri}"),"provider":"BLUESKY","remoteId":uri,"remoteUrl":format!("https://bsky.app/profile/{handle}/post/{rkey}"),"author":{"id":author["did"],"displayName":author["displayName"].as_str().unwrap_or(handle),"handle":handle,"avatarUrl":author["avatar"].as_str()},"text":record["text"].as_str().unwrap_or(""),"createdAt":record["createdAt"].as_str().unwrap_or(""),"media":media,"sources":[{"accountId":account_id,"accountHandle":account_handle,"provider":"BLUESKY"}],"metrics":{"replies":post["replyCount"],"reposts":post["repostCount"],"likes":post["likeCount"]},"capabilities":{"openOriginal":true,"reply":false,"like":false,"repost":false}})) }).collect::<Vec<_>>();
        Ok(serde_json::json!({"posts":posts,"cursor":native["cursor"].as_str()}))
    }
    async fn discovery(
        &self,
        _account_id: &str,
        _account_handle: &str,
    ) -> Result<serde_json::Value, AppError> {
        Ok(serde_json::json!({"topics":[],"suggestedAccounts":[],"popularPosts":[]}))
    }
    async fn following_sources(&self, account_id: &str) -> Result<serde_json::Value, AppError> {
        let session = self.session().await?;
        let value: serde_json::Value = self
            .client
            .get(format!(
                "{}/xrpc/app.bsky.actor.getPreferences",
                self.service_url.trim_end_matches('/')
            ))
            .bearer_auth(session.access_jwt)
            .send()
            .await
            .map_err(|e| AppError::Provider(format!("Bluesky preferences failed: {e}")))?
            .error_for_status()
            .map_err(|e| AppError::Provider(format!("Bluesky preferences failed: {e}")))?
            .json()
            .await
            .map_err(|e| AppError::Provider(format!("Invalid Bluesky preferences: {e}")))?;
        let sources=value["preferences"].as_array().into_iter().flatten().filter(|p|p["$type"]=="app.bsky.actor.defs#savedFeedsPrefV2").flat_map(|p|p["items"].as_array().into_iter().flatten()).filter(|i|i["type"]=="feed"&&i["pinned"].as_bool().unwrap_or(false)).filter_map(|i|i["value"].as_str()).map(|uri|serde_json::json!({"id":format!("BLUESKY:{uri}"),"provider":"BLUESKY","type":"FEED","title":uri.rsplit('/').next().unwrap_or("Saved feed"),"description":"Saved Bluesky feed","accountId":account_id,"remoteId":uri})).collect::<Vec<_>>();
        Ok(serde_json::json!(sources))
    }
    async fn hashtags(
        &self,
        query: &str,
    ) -> Result<Vec<crate::hashtags::HashtagSuggestion>, AppError> {
        self.search_hashtags(query).await
    }
    async fn capabilities(&self) -> Result<PlatformCapabilities, AppError> {
        Ok(self.capabilities.clone())
    }
    async fn publish(&self, post: PreparedPost) -> Result<PublishedPost, AppError> {
        self.create_record(post, None).await
    }
    async fn reply(
        &self,
        parent: &PublishedPost,
        post: PreparedPost,
    ) -> Result<PublishedPost, AppError> {
        self.create_record(post, Some(parent)).await
    }
}
