use serde::{Deserialize, Serialize};

use crate::models::ProviderKind;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProfileFeedKind {
    Posts,
    Replies,
    Media,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SourceKind {
    Person,
    Tag,
    List,
    Feed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NotificationKind {
    Mention,
    Reply,
    Like,
    Repost,
    Follow,
    Quote,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Actor {
    pub id: String,
    pub display_name: String,
    pub handle: String,
    pub avatar_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ViewerState {
    pub liked: bool,
    pub reposted: bool,
    pub like_uri: Option<String>,
    pub repost_uri: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PostMetrics {
    pub replies: Option<u64>,
    pub reposts: Option<u64>,
    pub likes: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SocialPost {
    pub canonical_key: String,
    pub provider: ProviderKind,
    pub remote_id: String,
    pub remote_cid: Option<String>,
    pub remote_url: String,
    pub author: Actor,
    pub text: String,
    pub created_at: String,
    pub media: Vec<Media>,
    pub metrics: PostMetrics,
    pub viewer: ViewerState,
    pub reply_parent_id: Option<String>,
    pub reply_root_id: Option<String>,
    pub reply_root_cid: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Media {
    pub url: String,
    pub alt: String,
    pub media_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeedPage {
    pub posts: Vec<SocialPost>,
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProfileDetails {
    pub actor: Actor,
    pub description: String,
    pub followers_count: Option<u64>,
    pub following_count: Option<u64>,
    pub posts_count: Option<u64>,
    pub followed_by_me: bool,
    pub follow_uri: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadView {
    pub ancestors: Vec<SocialPost>,
    pub post: SocialPost,
    pub replies: Vec<SocialPost>,
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FollowedSource {
    pub id: String,
    pub provider: ProviderKind,
    pub source_type: SourceKind,
    pub title: String,
    pub description: Option<String>,
    pub remote_id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourcePage {
    pub sources: Vec<FollowedSource>,
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationItem {
    pub id: String,
    pub kind: NotificationKind,
    pub created_at: String,
    pub actor: Actor,
    pub post: Option<SocialPost>,
    pub unread: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationPage {
    pub notifications: Vec<NotificationItem>,
    pub cursor: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "SCREAMING_SNAKE_CASE",
    rename_all_fields = "camelCase"
)]
pub enum SocialAction {
    Like { post_id: String },
    Unlike { post_id: String },
    Repost { post_id: String },
    UndoRepost { post_id: String },
    Follow { profile_id: String },
    Unfollow { profile_id: String },
    Reply { post_id: String, text: String },
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SocialActionResult {
    pub target_id: String,
    pub viewer: Option<ViewerState>,
    pub followed: Option<bool>,
    pub record_id: Option<String>,
    pub created_post: Option<SocialPost>,
}
