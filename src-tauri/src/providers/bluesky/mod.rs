//! Bluesky provider transport for app-password and OAuth accounts.
//!
//! App-password requests share a mutex-protected session cache for up to 20 minutes and briefly
//! cache failed sign-ins to avoid a request stampede. OAuth requests bypass that bearer-session
//! cache and use the persisted OAuth runtime, which owns refresh and authenticated transport.

mod discovery;
mod facets;
mod hashtags;
mod native;
mod normalize;
mod social_read;
mod social_sources;
mod social_write;
mod video;
use crate::{error::AppError, models::*, providers::SocialProvider};
use async_trait::async_trait;
use serde::Deserialize;
use std::sync::Arc;
use std::time::{Duration, Instant};
pub struct BlueskyProvider {
    pub(crate) app_password_session: tokio::sync::Mutex<AppPasswordSessionState>,
    pub capabilities: PlatformCapabilities,
    pub client: reqwest::Client,
    pub service_url: String,
    pub identifier: String,
    pub app_password: String,
    pub oauth: Option<Arc<crate::oauth::bluesky::BlueskyOAuthRuntime>>,
}

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SessionResponse {
    pub(crate) access_jwt: String,
    pub(crate) did: String,
    pub(crate) handle: String,
}

#[derive(Default)]
pub(crate) enum AppPasswordSessionState {
    #[default]
    Empty,
    Active {
        created: Instant,
        session: SessionResponse,
    },
    Failed {
        created: Instant,
    },
}
#[derive(Deserialize)]
struct RecordResponse {
    uri: String,
    cid: String,
}

impl BlueskyProvider {
    async fn create_session(&self) -> Result<SessionResponse, AppError> {
        if self.oauth.is_some() {
            return Err(AppError::Provider(
                "OAuth accounts do not expose bearer sessions".into(),
            ));
        }
        self.request(
            self.client.post(format!(
                "{}/xrpc/com.atproto.server.createSession",
                self.service_url.trim_end_matches('/')
            )),
            serde_json::json!({"identifier": self.identifier, "password": self.app_password}),
        )
        .await
    }

    async fn session(&self) -> Result<SessionResponse, AppError> {
        const CONSERVATIVE_SESSION_REUSE: Duration = Duration::from_secs(20 * 60);
        const FAILURE_COOLDOWN: Duration = Duration::from_secs(5);

        let mut state = self.app_password_session.lock().await;
        match &*state {
            AppPasswordSessionState::Active { created, session }
                if created.elapsed() < CONSERVATIVE_SESSION_REUSE =>
            {
                return Ok(session.clone());
            }
            AppPasswordSessionState::Failed { created } if created.elapsed() < FAILURE_COOLDOWN => {
                return Err(AppError::Provider(
                    "Bluesky sign-in recently failed; retry shortly".into(),
                ));
            }
            AppPasswordSessionState::Empty
            | AppPasswordSessionState::Active { .. }
            | AppPasswordSessionState::Failed { .. } => {}
        }
        match self.create_session().await {
            Ok(session) => {
                *state = AppPasswordSessionState::Active {
                    created: Instant::now(),
                    session: session.clone(),
                };
                Ok(session)
            }
            Err(error) => {
                *state = AppPasswordSessionState::Failed {
                    created: Instant::now(),
                };
                Err(error)
            }
        }
    }

    async fn invalidate_session(&self, access_jwt: &str) {
        let mut state = self.app_password_session.lock().await;
        if matches!(
            &*state,
            AppPasswordSessionState::Active { session, .. }
                if session.access_jwt == access_jwt
        ) {
            *state = AppPasswordSessionState::Empty;
        }
    }

    pub async fn account(&self) -> Result<(String, String), AppError> {
        if let Some(oauth) = &self.oauth {
            #[derive(Deserialize)]
            struct Profile {
                handle: String,
            }
            let profile: Profile = oauth
                .get_json("app.bsky.actor.getProfile", &[("actor", oauth.subject())])
                .await
                .map_err(oauth_error)?;
            return Ok((oauth.subject().into(), profile.handle));
        }
        let session = self.session().await?;
        Ok((session.did, session.handle))
    }

    pub(super) async fn account_did(&self) -> Result<String, AppError> {
        match &self.oauth {
            Some(oauth) => Ok(oauth.subject().into()),
            None => Ok(self.session().await?.did),
        }
    }

    pub(super) async fn get_json<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        query: &[(&str, &str)],
    ) -> Result<T, AppError> {
        self.get_json_with_timeout(path, query, Duration::from_secs(60))
            .await
    }

    pub(super) async fn get_json_with_timeout<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        query: &[(&str, &str)],
        timeout: Duration,
    ) -> Result<T, AppError> {
        if let Some(oauth) = &self.oauth {
            return oauth.get_json(path, query).await.map_err(oauth_error);
        }
        let mut session = self.session().await?;
        let mut response = self.get(path, query, &session.access_jwt, timeout).await?;
        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            self.invalidate_session(&session.access_jwt).await;
            session = self.session().await?;
            response = self.get(path, query, &session.access_jwt, timeout).await?;
        }
        response_json(response).await
    }

    async fn get(
        &self,
        path: &str,
        query: &[(&str, &str)],
        access_jwt: &str,
        timeout: Duration,
    ) -> Result<reqwest::Response, AppError> {
        self.client
            .get(format!(
                "{}/xrpc/{path}",
                self.service_url.trim_end_matches('/')
            ))
            .bearer_auth(access_jwt)
            .query(query)
            .timeout(timeout)
            .send()
            .await
            .map_err(|error| AppError::Provider(format!("Bluesky request failed: {error}")))
    }

    pub(super) async fn post_json<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        body: serde_json::Value,
    ) -> Result<T, AppError> {
        if let Some(oauth) = &self.oauth {
            return oauth.post_json(path, &body).await.map_err(oauth_error);
        }
        let session = self.session().await?;
        let access_jwt = session.access_jwt;
        let response = self
            .client
            .post(format!(
                "{}/xrpc/{path}",
                self.service_url.trim_end_matches('/')
            ))
            .bearer_auth(&access_jwt)
            .json(&body)
            .timeout(std::time::Duration::from_secs(60))
            .send()
            .await
            .map_err(|error| AppError::Provider(format!("Bluesky request failed: {error}")))?;
        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            self.invalidate_session(&access_jwt).await;
        }
        response_json(response).await
    }

    pub(super) async fn post_bytes<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        mime_type: &str,
        body: Vec<u8>,
    ) -> Result<T, AppError> {
        if let Some(oauth) = &self.oauth {
            return oauth
                .post_bytes(path, mime_type, body)
                .await
                .map_err(oauth_error);
        }
        let session = self.session().await?;
        let access_jwt = session.access_jwt;
        let response = self
            .client
            .post(format!(
                "{}/xrpc/{path}",
                self.service_url.trim_end_matches('/')
            ))
            .bearer_auth(&access_jwt)
            .header(reqwest::header::CONTENT_TYPE, mime_type)
            .body(body)
            .timeout(std::time::Duration::from_secs(60))
            .send()
            .await
            .map_err(|error| AppError::Provider(format!("Bluesky upload failed: {error}")))?;
        if response.status() == reqwest::StatusCode::UNAUTHORIZED {
            self.invalidate_session(&access_jwt).await;
        }
        response_json(response).await
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
        let app_session = if self.oauth.is_none() {
            Some(self.session().await?)
        } else {
            None
        };
        let did = match (&self.oauth, &app_session) {
            (Some(oauth), None) => oauth.subject().to_owned(),
            (None, Some(session)) => session.did.clone(),
            (Some(_), Some(_)) | (None, None) => return Err(AppError::StateUnavailable),
        };
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
        let video = post
            .media
            .iter()
            .find(|media| media.mime_type == "video/mp4");
        if video.is_some() && post.media.len() != 1 {
            return Err(AppError::Validation(
                "Bluesky posts may contain one video or images, but not both".into(),
            ));
        }
        if let Some(video) = video {
            let blob = match (&self.oauth, &app_session) {
                (Some(oauth), None) => {
                    video::upload_video_oauth(
                        oauth,
                        &self.client,
                        video::service_url(&self.service_url),
                        &did,
                        video,
                    )
                    .await?
                }
                (None, Some(session)) => {
                    video::upload_video(
                        &self.client,
                        &self.service_url,
                        video::service_url(&self.service_url),
                        &session.access_jwt,
                        &did,
                        video,
                    )
                    .await?
                }
                (Some(_), Some(_)) | (None, None) => return Err(AppError::StateUnavailable),
            };
            record["embed"] = serde_json::json!({
                "$type": "app.bsky.embed.video",
                "video": blob,
                "alt": video.alt_text,
            });
        } else if !post.media.is_empty() {
            let mut images = Vec::with_capacity(post.media.len());
            for image in &post.media {
                #[derive(Deserialize)]
                struct UploadResponse {
                    blob: serde_json::Value,
                }
                let uploaded: UploadResponse = match (&self.oauth, &app_session) {
                    (Some(_), None) => {
                        self.post_bytes(
                            "com.atproto.repo.uploadBlob",
                            &image.mime_type,
                            image.data.to_vec(),
                        )
                        .await?
                    }
                    (None, Some(session)) => {
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
                        response_json(response).await?
                    }
                    (Some(_), Some(_)) | (None, None) => return Err(AppError::StateUnavailable),
                };
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
        let body =
            serde_json::json!({"repo": did, "collection": "app.bsky.feed.post", "record": record});
        let response: RecordResponse = match (&self.oauth, &app_session) {
            (Some(_), None) => {
                self.post_json("com.atproto.repo.createRecord", body)
                    .await?
            }
            (None, Some(session)) => {
                self.request(
                    self.client
                        .post(format!(
                            "{}/xrpc/com.atproto.repo.createRecord",
                            self.service_url.trim_end_matches('/')
                        ))
                        .bearer_auth(&session.access_jwt),
                    body,
                )
                .await?
            }
            (Some(_), Some(_)) | (None, None) => return Err(AppError::StateUnavailable),
        };
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

fn oauth_error(error: crate::oauth::OAuthError) -> AppError {
    AppError::Provider(error.to_string())
}

async fn response_json<T: serde::de::DeserializeOwned>(
    response: reqwest::Response,
) -> Result<T, AppError> {
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
#[async_trait]
impl SocialProvider for BlueskyProvider {
    async fn home_feed(
        &self,
        cursor: Option<&str>,
    ) -> Result<crate::providers::social::FeedPage, AppError> {
        self.home_feed_page(cursor).await
    }
    async fn own_feed(
        &self,
        kind: crate::providers::social::ProfileFeedKind,
        cursor: Option<&str>,
    ) -> Result<crate::providers::social::FeedPage, AppError> {
        self.own_feed_page(kind, cursor).await
    }
    async fn own_profile(&self) -> Result<crate::providers::social::ProfileDetails, AppError> {
        self.own_profile_details().await
    }
    async fn profile(
        &self,
        profile_id: &str,
    ) -> Result<crate::providers::social::ProfileDetails, AppError> {
        self.profile_details(profile_id).await
    }
    async fn profile_feed(
        &self,
        profile_id: &str,
        kind: crate::providers::social::ProfileFeedKind,
        cursor: Option<&str>,
    ) -> Result<crate::providers::social::FeedPage, AppError> {
        self.profile_feed_page(profile_id, kind, cursor).await
    }
    async fn thread(
        &self,
        post_id: &str,
    ) -> Result<crate::providers::social::ThreadView, AppError> {
        self.thread_view(post_id).await
    }
    async fn tag_feed(
        &self,
        tag: &str,
        cursor: Option<&str>,
    ) -> Result<crate::providers::social::FeedPage, AppError> {
        self.tag_feed_page(tag, cursor).await
    }
    async fn followed_sources_page(
        &self,
        cursor: Option<&str>,
    ) -> Result<crate::providers::social::SourcePage, AppError> {
        self.followed_sources_page_impl(cursor).await
    }
    async fn source_feed(
        &self,
        source: &crate::providers::social::FollowedSource,
        cursor: Option<&str>,
    ) -> Result<crate::providers::social::FeedPage, AppError> {
        self.source_feed_page(source, cursor).await
    }
    async fn notifications(
        &self,
        cursor: Option<&str>,
    ) -> Result<crate::providers::social::NotificationPage, AppError> {
        self.notification_page(cursor).await
    }
    async fn mark_notifications_read(&self, ids: &[String]) -> Result<(), AppError> {
        self.mark_read(ids).await
    }
    async fn social_action(
        &self,
        action: crate::providers::social::SocialAction,
    ) -> Result<crate::providers::social::SocialActionResult, AppError> {
        self.apply_social_action(action).await
    }
    async fn timeline(
        &self,
        account_id: &str,
        account_handle: &str,
        cursor: Option<&str>,
    ) -> Result<serde_json::Value, AppError> {
        let mut query = vec![("limit", "50")];
        if let Some(cursor) = cursor {
            query.push(("cursor", cursor));
        }
        let native: serde_json::Value = self.get_json("app.bsky.feed.getTimeline", &query).await?;
        let posts = native["feed"].as_array().into_iter().flatten().filter_map(|item| { let post=&item["post"]; let uri=post["uri"].as_str()?; let author=&post["author"]; let record=&post["record"]; let handle=author["handle"].as_str().unwrap_or(""); let rkey=uri.rsplit('/').next().unwrap_or(""); let media=item["post"]["embed"]["images"].as_array().map(|images| images.iter().map(|image| serde_json::json!({"url":image["fullsize"],"alt":image["alt"].as_str().unwrap_or(""),"type":"image"})).collect::<Vec<_>>()).unwrap_or_default(); Some(serde_json::json!({"canonicalKey":format!("BLUESKY:{uri}"),"provider":"BLUESKY","remoteId":uri,"remoteUrl":format!("https://bsky.app/profile/{handle}/post/{rkey}"),"author":{"id":author["did"],"displayName":author["displayName"].as_str().unwrap_or(handle),"handle":handle,"avatarUrl":author["avatar"].as_str()},"text":record["text"].as_str().unwrap_or(""),"createdAt":record["createdAt"].as_str().unwrap_or(""),"media":media,"sources":[{"accountId":account_id,"accountHandle":account_handle,"provider":"BLUESKY"}],"metrics":{"replies":post["replyCount"],"reposts":post["repostCount"],"likes":post["likeCount"]},"capabilities":{"openOriginal":true,"reply":false,"like":false,"repost":false}})) }).collect::<Vec<_>>();
        Ok(serde_json::json!({"posts":posts,"cursor":native["cursor"].as_str()}))
    }
    async fn discovery(
        &self,
        account_id: &str,
        account_handle: &str,
    ) -> Result<serde_json::Value, AppError> {
        self.discovery_result(account_id, account_handle).await
    }
    async fn following_sources(&self, account_id: &str) -> Result<serde_json::Value, AppError> {
        Ok(crate::providers::legacy_sources(
            account_id,
            self.followed_sources_page_impl(None).await?.sources,
        ))
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
