use crate::{error::AppError, models::*, providers::SocialProvider};
use async_trait::async_trait;
use serde::Deserialize;
pub struct BlueskyProvider {
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
