pub mod bluesky;
pub mod mastodon;
use crate::{
    error::AppError,
    models::{PlatformCapabilities, PreparedPost, PublishedPost},
};
use async_trait::async_trait;
#[async_trait]
pub trait SocialProvider: Send + Sync {
    async fn capabilities(&self) -> Result<PlatformCapabilities, AppError>;
    async fn publish(&self, post: PreparedPost) -> Result<PublishedPost, AppError>;
    async fn reply(
        &self,
        parent: &PublishedPost,
        post: PreparedPost,
    ) -> Result<PublishedPost, AppError>;
}
