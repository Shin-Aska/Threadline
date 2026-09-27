//! Response shapes consumed from Bluesky App View endpoints.
//!
//! These structs retain provider identifiers and viewer record URIs until the
//! normalization layer converts them to provider-neutral social models.

use serde::Deserialize;

/// An account profile returned by the App View.
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Actor {
    pub did: String,
    pub handle: String,
    pub display_name: Option<String>,
    pub avatar: Option<String>,
    pub description: Option<String>,
    pub followers_count: Option<u64>,
    pub follows_count: Option<u64>,
    pub posts_count: Option<u64>,
    pub viewer: Option<ActorViewer>,
}

/// The current account's relationship to an actor.
#[derive(Clone, Deserialize)]
pub(super) struct ActorViewer {
    /// URI of the current account's follow record, when present.
    pub following: Option<String>,
}

/// An indexed post with its strong reference, author, metrics, and viewer state.
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct PostView {
    pub uri: String,
    pub cid: String,
    pub author: Actor,
    pub record: PostRecord,
    pub embed: Option<EmbedView>,
    pub reply_count: Option<u64>,
    pub repost_count: Option<u64>,
    pub like_count: Option<u64>,
    pub viewer: Option<PostViewer>,
}

/// Post content and optional reply references from the underlying record.
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct PostRecord {
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub created_at: String,
    pub reply: Option<ReplyRef>,
}

/// References to a reply's thread root and immediate parent.
#[derive(Clone, Deserialize)]
pub(super) struct ReplyRef {
    pub root: StrongRef,
    pub parent: StrongRef,
}

/// AT Protocol URI and content identifier for a specific record version.
#[derive(Clone, Deserialize)]
pub(super) struct StrongRef {
    pub uri: String,
    pub cid: String,
}

/// URIs of the current account's like and repost records, when they exist.
#[derive(Clone, Deserialize)]
pub(super) struct PostViewer {
    pub like: Option<String>,
    pub repost: Option<String>,
}

/// Image data exposed by a post embed in the consumed response shape.
#[derive(Clone, Deserialize)]
pub(super) struct EmbedView {
    pub images: Option<Vec<ImageView>>,
}

/// Full-size image URL and its accessibility text.
#[derive(Clone, Deserialize)]
pub(super) struct ImageView {
    pub fullsize: String,
    #[serde(default)]
    pub alt: String,
}

/// Feed entry containing the indexed post used by normalization.
#[derive(Deserialize)]
pub(super) struct FeedItem {
    pub post: PostView,
}

/// A paginated timeline, author feed, list, or custom feed response.
#[derive(Deserialize)]
pub(super) struct FeedResponse {
    pub cursor: Option<String>,
    #[serde(default)]
    pub feed: Vec<FeedItem>,
}

/// A paginated set of posts from search or URI lookup.
#[derive(Deserialize)]
pub(super) struct PostsResponse {
    pub cursor: Option<String>,
    #[serde(default)]
    pub posts: Vec<PostView>,
}

/// Root entry of a fetched post thread.
#[derive(Deserialize)]
pub(super) struct ThreadResponse {
    pub thread: ThreadEntry,
}

/// Recursive thread node; unavailable posts have no `post` value.
#[derive(Deserialize)]
pub(super) struct ThreadEntry {
    pub post: Option<PostView>,
    pub parent: Option<Box<ThreadEntry>>,
    #[serde(default)]
    pub replies: Vec<ThreadEntry>,
}

/// A page of accounts followed by a Bluesky user.
#[derive(Deserialize)]
pub(super) struct ActorsResponse {
    pub cursor: Option<String>,
    #[serde(default)]
    pub follows: Vec<Actor>,
}

/// Lists returned for the connected account.
#[derive(Deserialize)]
pub(super) struct ListsResponse {
    #[serde(default)]
    pub lists: Vec<ListView>,
}

/// Identity and display metadata for one Bluesky list.
#[derive(Deserialize)]
pub(super) struct ListView {
    pub uri: String,
    pub name: String,
    pub description: Option<String>,
}

/// Saved preferences that can contain custom feed subscriptions.
#[derive(Deserialize)]
pub(super) struct PreferencesResponse {
    #[serde(default)]
    pub preferences: Vec<Preference>,
}

/// Typed preference record; saved feeds are read from `items`.
#[derive(Deserialize)]
pub(super) struct Preference {
    #[serde(rename = "$type")]
    pub record_type: String,
    pub items: Option<Vec<SavedFeed>>,
}

/// Feed subscription identifier and pinned state.
#[derive(Deserialize)]
pub(super) struct SavedFeed {
    #[serde(rename = "type")]
    pub item_type: String,
    pub value: String,
    #[serde(default)]
    pub pinned: bool,
}

/// A page of inbox events and its continuation cursor.
#[derive(Deserialize)]
pub(super) struct NotificationResponse {
    pub cursor: Option<String>,
    #[serde(default)]
    pub notifications: Vec<Notification>,
}

/// Inbox event whose reason can refer to an embedded or separate post.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Notification {
    pub uri: String,
    pub cid: String,
    pub author: Actor,
    pub reason: String,
    /// Post URI fetched separately for likes, reposts, and similar events.
    pub reason_subject: Option<String>,
    pub record: PostRecord,
    pub is_read: bool,
    pub indexed_at: String,
}

/// Accounts suggested by the Bluesky actor endpoint.
#[derive(Deserialize)]
pub(super) struct SuggestionsResponse {
    #[serde(default)]
    pub actors: Vec<Actor>,
}

/// Current and suggested topic lists from the discovery endpoint.
#[derive(Deserialize)]
pub(super) struct TrendingTopicsResponse {
    #[serde(default)]
    pub topics: Vec<TrendingTopic>,
    #[serde(default)]
    pub suggested: Vec<TrendingTopic>,
}

/// Topic key and optional display label used by discovery.
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct TrendingTopic {
    pub topic: String,
    pub display_name: Option<String>,
}
