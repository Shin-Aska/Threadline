use super::MastodonProvider;
use crate::{error::AppError, models::PreparedMedia};
use serde::Deserialize;
use std::time::Duration;

#[derive(Deserialize)]
struct UploadedMedia {
    id: String,
    url: Option<String>,
}

async fn read_media(response: reqwest::Response) -> Result<UploadedMedia, AppError> {
    let status = response.status();
    if !status.is_success() {
        return Err(AppError::Provider(format!(
            "Mastodon media upload returned {status}"
        )));
    }
    response
        .json()
        .await
        .map_err(|_| AppError::Provider("Invalid Mastodon media response".into()))
}

impl MastodonProvider {
    pub(super) async fn upload_media(
        &self,
        media_items: &[PreparedMedia],
    ) -> Result<Vec<String>, AppError> {
        let mut ids = Vec::with_capacity(media_items.len());
        for media in media_items {
            let extension = media.mime_type.split('/').nth(1).unwrap_or("bin");
            let file = reqwest::multipart::Part::bytes(media.data.to_vec())
                .file_name(format!("attachment.{extension}"))
                .mime_str(&media.mime_type)
                .map_err(|_| AppError::Validation("unsupported media type".into()))?;
            let response = self
                .client
                .post(format!(
                    "{}/api/v2/media",
                    self.base_url.trim_end_matches('/')
                ))
                .bearer_auth(&self.access_token)
                .timeout(Duration::from_secs(60))
                .multipart(
                    reqwest::multipart::Form::new()
                        .part("file", file)
                        .text("description", media.alt_text.clone()),
                )
                .send()
                .await
                .map_err(|error| {
                    AppError::Provider(format!("Mastodon media upload failed: {error}"))
                })?;
            if !response.status().is_success() {
                let media_kind = if media.mime_type == "video/mp4" {
                    "video"
                } else {
                    "image"
                };
                return Err(AppError::Provider(format!(
                    "Mastodon {media_kind} upload returned {}",
                    response.status()
                )));
            }
            let mut uploaded = read_media(response).await?;
            let processing_attempts = if media.mime_type == "video/mp4" {
                600
            } else {
                30
            };
            for _ in 0..processing_attempts {
                if uploaded.url.is_some() {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(500)).await;
                let response = self
                    .client
                    .get(format!(
                        "{}/api/v1/media/{}",
                        self.base_url.trim_end_matches('/'),
                        uploaded.id
                    ))
                    .bearer_auth(&self.access_token)
                    .timeout(Duration::from_secs(15))
                    .send()
                    .await
                    .map_err(|error| {
                        AppError::Provider(format!(
                            "Mastodon media processing check failed: {error}"
                        ))
                    })?;
                if response.status() == reqwest::StatusCode::PARTIAL_CONTENT {
                    continue;
                }
                uploaded = read_media(response).await?;
            }
            if uploaded.url.is_none() {
                return Err(AppError::Provider("Mastodon is still processing media. Your post was not sent; try again shortly.".into()));
            }
            ids.push(uploaded.id);
        }
        Ok(ids)
    }
}
