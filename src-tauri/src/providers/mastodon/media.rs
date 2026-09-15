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
            "Mastodon image upload returned {status}"
        )));
    }
    response
        .json()
        .await
        .map_err(|_| AppError::Provider("Invalid Mastodon media response".into()))
}

impl MastodonProvider {
    pub(super) async fn upload_images(
        &self,
        images: &[PreparedMedia],
    ) -> Result<Vec<String>, AppError> {
        let mut ids = Vec::with_capacity(images.len());
        for image in images {
            let extension = image.mime_type.strip_prefix("image/").unwrap_or("bin");
            let file = reqwest::multipart::Part::bytes(image.data.to_vec())
                .file_name(format!("image.{extension}"))
                .mime_str(&image.mime_type)
                .map_err(|_| AppError::Validation("unsupported image type".into()))?;
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
                        .text("description", image.alt_text.clone()),
                )
                .send()
                .await
                .map_err(|error| {
                    AppError::Provider(format!("Mastodon image upload failed: {error}"))
                })?;
            let mut uploaded = read_media(response).await?;
            for _ in 0..30 {
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
                            "Mastodon image processing check failed: {error}"
                        ))
                    })?;
                if response.status() == reqwest::StatusCode::PARTIAL_CONTENT {
                    continue;
                }
                uploaded = read_media(response).await?;
            }
            if uploaded.url.is_none() {
                return Err(AppError::Provider("Mastodon is still processing an image. Your post was not sent; try again shortly.".into()));
            }
            ids.push(uploaded.id);
        }
        Ok(ids)
    }
}
