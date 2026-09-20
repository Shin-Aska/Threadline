use crate::{
    error::AppError,
    models::PublishedPost,
    providers::social::{SocialAction, SocialActionResult, ViewerState},
};

use super::{native, normalize, MastodonProvider};

impl MastodonProvider {
    async fn social_post<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        form: &[(&str, &str)],
    ) -> Result<T, AppError> {
        let response = self
            .client
            .post(format!("{}{}", self.base_url, path))
            .bearer_auth(&self.access_token)
            .timeout(std::time::Duration::from_secs(60))
            .form(form)
            .send()
            .await
            .map_err(|error| AppError::Provider(format!("Mastodon action failed: {error}")))?;
        let status = response.status();
        let body = response.text().await.map_err(|error| {
            AppError::Provider(format!(
                "Mastodon action response could not be read: {error}"
            ))
        })?;
        if !status.is_success() {
            return Err(AppError::Provider(format!(
                "Mastodon action returned {status}: {}",
                crate::providers::safe_error_body(&body)
            )));
        }
        serde_json::from_str(&body).map_err(|error| {
            AppError::Provider(format!("Invalid Mastodon action response: {error}"))
        })
    }

    pub(super) async fn apply_social_action(
        &self,
        action: SocialAction,
    ) -> Result<SocialActionResult, AppError> {
        match action {
            SocialAction::Like { post_id, .. } => {
                self.status_action(&post_id, "favourite", true).await
            }
            SocialAction::Unlike { post_id, .. } => {
                self.status_action(&post_id, "unfavourite", false).await
            }
            SocialAction::Repost { post_id, .. } => {
                self.status_action(&post_id, "reblog", true).await
            }
            SocialAction::UndoRepost { post_id, .. } => {
                self.status_action(&post_id, "unreblog", false).await
            }
            SocialAction::Follow { profile_id } => {
                self.follow_action(&profile_id, "follow", true).await
            }
            SocialAction::Unfollow { profile_id } => {
                self.follow_action(&profile_id, "unfollow", false).await
            }
            SocialAction::Reply { post_id, text } => {
                let published = self
                    .create_status(
                        crate::models::PreparedPost {
                            text,
                            media: Vec::new(),
                        },
                        Some(&post_id),
                    )
                    .await?;
                self.reply_result(post_id, published).await
            }
        }
    }

    async fn status_action(
        &self,
        post_id: &str,
        operation: &str,
        active: bool,
    ) -> Result<SocialActionResult, AppError> {
        let status: native::Status = self
            .social_post(&format!("/api/v1/statuses/{post_id}/{operation}"), &[])
            .await?;
        Ok(SocialActionResult {
            target_id: post_id.to_owned(),
            viewer: Some(ViewerState {
                liked: status.favourited,
                reposted: status.reblogged,
                like_uri: None,
                repost_uri: None,
            }),
            followed: None,
            record_id: active.then(|| status.id.clone()),
            created_post: Some(normalize::post(&self.base_url, status)?),
        })
    }

    async fn follow_action(
        &self,
        profile_id: &str,
        operation: &str,
        followed: bool,
    ) -> Result<SocialActionResult, AppError> {
        let relationship: native::Relationship = self
            .social_post(&format!("/api/v1/accounts/{profile_id}/{operation}"), &[])
            .await?;
        Ok(SocialActionResult {
            target_id: profile_id.to_owned(),
            viewer: None,
            followed: Some(relationship.following && followed),
            record_id: None,
            created_post: None,
        })
    }

    async fn reply_result(
        &self,
        parent_id: String,
        published: PublishedPost,
    ) -> Result<SocialActionResult, AppError> {
        let status: native::Status = self
            .social_get(
                self.client.get(format!(
                    "{}/api/v1/statuses/{}",
                    self.base_url, published.remote_id
                )),
                "reply",
            )
            .await?;
        Ok(SocialActionResult {
            target_id: parent_id,
            viewer: None,
            followed: None,
            record_id: Some(published.remote_id),
            created_post: Some(normalize::post(&self.base_url, status)?),
        })
    }
}
