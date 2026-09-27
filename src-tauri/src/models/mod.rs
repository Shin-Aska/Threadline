//! Shared account, composer, and publishing data passed between the backend and UI.
//!
//! Serde naming attributes define the JSON contract; provider transports use
//! the prepared types at the bottom of this module after validation and splitting.

use serde::{Deserialize, Serialize};
pub mod publishing;
pub use publishing::*;
/// Social network that owns an account or remote post.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProviderKind {
    /// A Mastodon server in the fediverse.
    Mastodon,
    /// The Bluesky AT Protocol network.
    Bluesky,
}
/// Rule used to measure a provider's post text limit.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum CountingPolicy {
    /// Count user-perceived Unicode grapheme clusters.
    Grapheme,
    /// Apply the provider's own text counting convention.
    PlatformNative,
}
/// Text, media, and feature limits reported for one publishing destination.
///
/// Optional video limits and reserved URL length are absent when unknown or
/// inapplicable. The composer uses these values to preview destination posts.
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
/// One saved social account shown in the workspace.
///
/// `id` is Threadline's account key. Mastodon accounts may include an instance
/// URL, while Bluesky accounts may include a DID; the provider connection and
/// credentials are managed separately from this serializable record.
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
/// Whether any live provider connection is currently available.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum WorkspaceMode {
    /// At least one account has a usable provider connection.
    Live,
    /// No provider connection is currently available.
    Disconnected,
}
/// Account list and connection status sent to the workspace UI.
///
/// `connected_account_ids` names accounts with live provider objects;
/// `accounts` may also contain stored accounts that need reconnection.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceState {
    pub accounts: Vec<Account>,
    pub connected_account_ids: Vec<String>,
    pub mode: WorkspaceMode,
}
/// Strategy for splitting a canonical post across destination limits.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PublishingPolicy {
    /// Split every destination using the smallest selected text limit.
    CommonLimit,
    /// Split each destination using its own text limit.
    Adaptive,
    /// Number a thread even when the post fits in one part.
    AlwaysThread,
}
/// User-selected media before validation and provider upload.
///
/// `data_base64` carries file bytes in incoming requests. Draft persistence
/// stores media separately and restores this field when a draft is loaded.
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
/// One authoring intent shared by all selected destination accounts.
///
/// The composer derives provider-specific text parts from `text` and `policy`;
/// this type preserves the original text, media, and destination selection.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CanonicalPost {
    pub text: String,
    pub media: Vec<MediaAttachment>,
    pub policy: PublishingPolicy,
    pub destination_account_ids: Vec<String>,
}
/// How a canonical post would be split for one destination.
///
/// `parts` are the text payloads to publish in order; `max_length` is the
/// destination's own limit even when a shared policy applies a smaller one.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DestinationPreview {
    pub account_id: String,
    pub label: String,
    pub max_length: usize,
    pub parts: Vec<String>,
}
/// Composer preview covering every selected destination.
///
/// `effective_limit` and `limiting_account_id` identify the shared constraint
/// when the chosen publishing policy has one.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishingPreview {
    pub grapheme_count: usize,
    pub effective_limit: Option<usize>,
    pub limiting_account_id: Option<String>,
    pub destinations: Vec<DestinationPreview>,
}
/// Summary status for one destination in the immediate publication result.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PublicationStatus {
    /// The provider confirmed publication.
    Published,
    /// Publication failed before a confirmed remote post was returned.
    Failed,
    /// The provider outcome is ambiguous and needs user review.
    Uncertain,
    /// The destination cannot currently be published to.
    Blocked,
}
/// Immediate publication result for one selected account.
///
/// Remote IDs are present for confirmed posts; `error` explains a non-success
/// status when the publication path can provide one.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Publication {
    pub account_id: String,
    pub status: PublicationStatus,
    pub remote_post_ids: Vec<String>,
    pub error: Option<String>,
}
/// Immediate multi-account publication result for one canonical post.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PublishResult {
    pub canonical_id: String,
    pub publications: Vec<Publication>,
}
/// One provider-ready text part with media already decoded and validated.
#[derive(Debug, Clone)]
pub struct PreparedPost {
    pub text: String,
    pub media: Vec<PreparedMedia>,
}
/// Binary attachment passed to a provider upload operation.
///
/// The bytes are shared with `Arc` so the same prepared media can be reused
/// while publishing to multiple destinations.
#[derive(Debug, Clone)]
pub struct PreparedMedia {
    pub mime_type: String,
    pub data: std::sync::Arc<[u8]>,
    pub alt_text: String,
}
/// Identifiers returned after a provider confirms one published post.
///
/// A provider can use the optional content and thread-root identifiers to
/// construct later replies in the same thread.
#[derive(Debug, Clone)]
pub struct PublishedPost {
    pub remote_id: String,
    pub remote_cid: Option<String>,
    pub root_id: Option<String>,
    pub root_cid: Option<String>,
}
