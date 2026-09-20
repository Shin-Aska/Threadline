use serde::Deserialize;

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

#[derive(Clone, Deserialize)]
pub(super) struct ActorViewer {
    pub following: Option<String>,
}

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

#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct PostRecord {
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub created_at: String,
    pub reply: Option<ReplyRef>,
}

#[derive(Clone, Deserialize)]
pub(super) struct ReplyRef {
    pub root: StrongRef,
    pub parent: StrongRef,
}

#[derive(Clone, Deserialize)]
pub(super) struct StrongRef {
    pub uri: String,
    pub cid: String,
}

#[derive(Clone, Deserialize)]
pub(super) struct PostViewer {
    pub like: Option<String>,
    pub repost: Option<String>,
}

#[derive(Clone, Deserialize)]
pub(super) struct EmbedView {
    pub images: Option<Vec<ImageView>>,
}

#[derive(Clone, Deserialize)]
pub(super) struct ImageView {
    pub fullsize: String,
    #[serde(default)]
    pub alt: String,
}

#[derive(Deserialize)]
pub(super) struct FeedItem {
    pub post: PostView,
}

#[derive(Deserialize)]
pub(super) struct FeedResponse {
    pub cursor: Option<String>,
    #[serde(default)]
    pub feed: Vec<FeedItem>,
}

#[derive(Deserialize)]
pub(super) struct PostsResponse {
    pub cursor: Option<String>,
    #[serde(default)]
    pub posts: Vec<PostView>,
}

#[derive(Deserialize)]
pub(super) struct ThreadResponse {
    pub thread: ThreadEntry,
}

#[derive(Deserialize)]
pub(super) struct ThreadEntry {
    pub post: Option<PostView>,
    pub parent: Option<Box<ThreadEntry>>,
    #[serde(default)]
    pub replies: Vec<ThreadEntry>,
}

#[derive(Deserialize)]
pub(super) struct ActorsResponse {
    pub cursor: Option<String>,
    #[serde(default)]
    pub follows: Vec<Actor>,
}

#[derive(Deserialize)]
pub(super) struct ListsResponse {
    #[serde(default)]
    pub lists: Vec<ListView>,
}

#[derive(Deserialize)]
pub(super) struct ListView {
    pub uri: String,
    pub name: String,
    pub description: Option<String>,
}

#[derive(Deserialize)]
pub(super) struct PreferencesResponse {
    #[serde(default)]
    pub preferences: Vec<Preference>,
}

#[derive(Deserialize)]
pub(super) struct Preference {
    #[serde(rename = "$type")]
    pub record_type: String,
    pub items: Option<Vec<SavedFeed>>,
}

#[derive(Deserialize)]
pub(super) struct SavedFeed {
    #[serde(rename = "type")]
    pub item_type: String,
    pub value: String,
    #[serde(default)]
    pub pinned: bool,
}

#[derive(Deserialize)]
pub(super) struct NotificationResponse {
    pub cursor: Option<String>,
    #[serde(default)]
    pub notifications: Vec<Notification>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct Notification {
    pub uri: String,
    pub cid: String,
    pub author: Actor,
    pub reason: String,
    pub reason_subject: Option<String>,
    pub record: PostRecord,
    pub is_read: bool,
    pub indexed_at: String,
}

#[derive(Deserialize)]
pub(super) struct SuggestionsResponse {
    #[serde(default)]
    pub actors: Vec<Actor>,
}

#[derive(Deserialize)]
pub(super) struct TrendingTopicsResponse {
    #[serde(default)]
    pub topics: Vec<TrendingTopic>,
    #[serde(default)]
    pub suggested: Vec<TrendingTopic>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct TrendingTopic {
    pub topic: String,
    pub display_name: Option<String>,
}
