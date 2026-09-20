use serde::{Deserialize, Serialize};
pub mod publishing;
pub use publishing::*;
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProviderKind {
    Mastodon,
    Bluesky,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CountingPolicy {
    Grapheme,
    PlatformNative,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlatformCapabilities {
    pub max_text_length: usize,
    pub counting_policy: CountingPolicy,
    pub reserved_url_length: Option<usize>,
    pub max_media_attachments: usize,
    pub supported_media_types: Vec<String>,
    #[serde(default)]
    pub max_video_bytes: Option<usize>,
    #[serde(default)]
    pub max_video_duration_ms: Option<u64>,
    pub supports_polls: bool,
    pub supports_content_warnings: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Account {
    pub id: String,
    pub provider: ProviderKind,
    pub handle: String,
    pub display_name: String,
    pub instance_url: Option<String>,
    pub did: Option<String>,
    pub capabilities: PlatformCapabilities,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WorkspaceMode {
    Live,
    Disconnected,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceState {
    pub accounts: Vec<Account>,
    pub connected_account_ids: Vec<String>,
    pub mode: WorkspaceMode,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PublishingPolicy {
    CommonLimit,
    Adaptive,
    AlwaysThread,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct MediaAttachment {
    pub id: String,
    #[serde(default)]
    pub name: String,
    pub mime_type: String,
    pub size_bytes: usize,
    pub alt_text: String,
    #[serde(default)]
    pub data_base64: String,
    #[serde(default)]
    pub duration_ms: Option<u64>,
}
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CanonicalPost {
    pub text: String,
    pub media: Vec<MediaAttachment>,
    pub policy: PublishingPolicy,
    pub destination_account_ids: Vec<String>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DestinationPreview {
    pub account_id: String,
    pub label: String,
    pub max_length: usize,
    pub parts: Vec<String>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishingPreview {
    pub grapheme_count: usize,
    pub effective_limit: Option<usize>,
    pub limiting_account_id: Option<String>,
    pub destinations: Vec<DestinationPreview>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PublicationStatus {
    Published,
    Failed,
    Uncertain,
    Blocked,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Publication {
    pub account_id: String,
    pub status: PublicationStatus,
    pub remote_post_ids: Vec<String>,
    pub error: Option<String>,
}
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishResult {
    pub canonical_id: String,
    pub publications: Vec<Publication>,
}
#[derive(Debug, Clone)]
pub struct PreparedPost {
    pub text: String,
    pub media: Vec<PreparedMedia>,
}
#[derive(Debug, Clone)]
pub struct PreparedMedia {
    pub mime_type: String,
    pub data: std::sync::Arc<[u8]>,
    pub alt_text: String,
}
#[derive(Debug, Clone)]
pub struct PublishedPost {
    pub remote_id: String,
    pub remote_cid: Option<String>,
    pub root_id: Option<String>,
    pub root_cid: Option<String>,
}
