use crate::{error::AppError, models::*, providers::SocialProvider};
use async_trait::async_trait;
pub struct MastodonProvider {
    pub capabilities: PlatformCapabilities,
    pub client: reqwest::Client,
}
#[async_trait]
impl SocialProvider for MastodonProvider {
    async fn capabilities(&self) -> Result<PlatformCapabilities, AppError> {
        Ok(self.capabilities.clone())
    }
    async fn publish(&self, _: PreparedPost) -> Result<PublishedPost, AppError> {
        Err(AppError::Provider(
            "Mastodon networking is not implemented in this milestone".into(),
        ))
    }
    async fn reply(&self, _: &PublishedPost, _: PreparedPost) -> Result<PublishedPost, AppError> {
        Err(AppError::Provider(
            "Mastodon networking is not implemented in this milestone".into(),
        ))
    }
}
