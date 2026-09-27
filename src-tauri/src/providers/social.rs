//! Provider-neutral models exchanged between social transports and commands.
//!
//! These types let Bluesky and Mastodon expose the same feed and action API.
//! Serde renames enum values and struct fields to match the frontend payloads.

use serde::{Deserialize, Serialize};

use crate::models::ProviderKind;

/// Selects which portion of a profile's activity to load.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ProfileFeedKind {
    /// Posts authored by the profile.
    Posts,
    /// Replies authored by the profile.
    Replies,
    /// Posts from the profile that include media.
    Media,
}

/// Identifies the kind of account, tag, or collection being followed.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SourceKind {
    /// An individual account.
    Person,
    /// A followed hashtag or topic.
    Tag,
    /// A provider list of accounts.
    List,
    /// A provider-defined feed.
    Feed,
}

/// Categorizes an activity shown in the connected account's notifications.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum NotificationKind {
    /// The account was mentioned in a post.
    Mention,
    /// Someone replied to the account's post.
    Reply,
    /// Someone liked the account's post.
    Like,
    /// Someone reposted the account's post.
    Repost,
    /// Someone followed the account.
    Follow,
    /// Someone quoted the account's post.
    Quote,
    /// A provider event without a more specific shared category.
    Other,
}

/// Public identity information for a post author or notification actor.
///
/// `id` is the provider's remote identity; `handle` is its displayable handle.
/// The avatar URL may be absent when the provider does not supply one.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Actor {
    pub id: String,
    pub display_name: String,
    pub handle: String,
    pub avatar_url: Option<String>,
}

/// Relationship of the connected account to a particular post.
///
/// The optional record URIs identify the account's existing like or repost,
/// which a provider may need when undoing that action.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct ViewerState {
    pub liked: bool,
    pub reposted: bool,
    pub like_uri: Option<String>,
    pub repost_uri: Option<String>,
}

/// Engagement counts supplied by a provider for one post.
///
/// Each count is optional because a provider response may omit that metric.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PostMetrics {
    pub replies: Option<u64>,
    pub reposts: Option<u64>,
    pub likes: Option<u64>,
}

/// A normalized social post, independent of the originating provider's API.
///
/// `canonical_key` supports identifying the same post across account feeds.
/// `remote_id` is the provider's post identifier, while `remote_cid` carries
/// a content identifier when one is available. Reply IDs and the root CID are
/// optional because a post may be standalone or its provider may omit them.
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

/// One media attachment displayed with a normalized post.
///
/// `url` locates the attachment, `alt` is its accessibility description, and
/// `media_type` describes the attachment's format.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Media {
    pub url: String,
    pub alt: String,
    pub media_type: String,
}

/// A page of normalized posts and a cursor for requesting the next page.
///
/// A missing cursor means the provider supplied no next-page token.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FeedPage {
    pub posts: Vec<SocialPost>,
    pub cursor: Option<String>,
}

/// A profile and the connected account's relationship to it.
///
/// Counts can be absent when a provider does not return them. `follow_uri`
/// identifies an existing follow record when one is available, allowing the
/// provider to undo that follow.
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

/// A conversation centered on one post.
///
/// `ancestors` precede the focal `post`; `replies` are its loaded responses.
/// `cursor` can request more replies if the provider supplies one.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ThreadView {
    pub ancestors: Vec<SocialPost>,
    pub post: SocialPost,
    pub replies: Vec<SocialPost>,
    pub cursor: Option<String>,
}

/// A person, topic, list, or feed followed by the connected account.
///
/// `id` identifies this source within the app, while `remote_id` identifies
/// it to its provider. `source_type` determines how its posts are fetched.
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

/// A page of followed sources with an optional next-page cursor.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SourcePage {
    pub sources: Vec<FollowedSource>,
    pub cursor: Option<String>,
}

/// One activity event in the connected account's notification inbox.
///
/// `actor` performed the activity; `post` is optional because some events,
/// such as a follow, do not include a related post.
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

/// A page of notifications with an optional next-page cursor.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NotificationPage {
    pub notifications: Vec<NotificationItem>,
    pub cursor: Option<String>,
}

/// A user-requested operation on a post or profile.
///
/// Serde uses the `kind` field to select a variant, with uppercase action
/// names and camel-case fields in the frontend payload.
#[derive(Debug, Clone, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "SCREAMING_SNAKE_CASE",
    rename_all_fields = "camelCase"
)]
pub enum SocialAction {
    /// Like the post identified by `post_id`.
    Like { post_id: String },
    /// Remove the connected account's like from a post.
    Unlike { post_id: String },
    /// Repost the post identified by `post_id`.
    Repost { post_id: String },
    /// Remove the connected account's repost of a post.
    UndoRepost { post_id: String },
    /// Follow the profile identified by `profile_id`.
    Follow { profile_id: String },
    /// Stop following the profile identified by `profile_id`.
    Unfollow { profile_id: String },
    /// Publish `text` as a reply to `post_id`.
    Reply { post_id: String, text: String },
}

/// Updated state returned after a social action completes.
///
/// `target_id` identifies the affected post or profile. Optional fields carry
/// the state relevant to that action: post viewer state, follow state, a
/// created provider record, or a newly published reply.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SocialActionResult {
    pub target_id: String,
    pub viewer: Option<ViewerState>,
    pub followed: Option<bool>,
    pub record_id: Option<String>,
    pub created_post: Option<SocialPost>,
}
