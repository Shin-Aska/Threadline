use serde::Deserialize;

#[derive(Clone, Deserialize)]
pub(super) struct Account {
    pub id: String,
    pub acct: String,
    #[serde(default)]
    pub display_name: String,
    pub avatar: Option<String>,
    #[serde(default)]
    pub note: String,
    pub followers_count: Option<u64>,
    pub following_count: Option<u64>,
    pub statuses_count: Option<u64>,
}

#[derive(Clone, Deserialize)]
pub(super) struct Attachment {
    pub url: String,
    pub description: Option<String>,
    #[serde(rename = "type")]
    pub media_type: String,
}

#[derive(Clone, Deserialize)]
pub(super) struct Status {
    pub id: String,
    pub url: Option<String>,
    pub created_at: String,
    #[serde(default)]
    pub content: String,
    pub account: Account,
    #[serde(default)]
    pub media_attachments: Vec<Attachment>,
    pub replies_count: Option<u64>,
    pub reblogs_count: Option<u64>,
    pub favourites_count: Option<u64>,
    #[serde(default)]
    pub favourited: bool,
    #[serde(default)]
    pub reblogged: bool,
    pub in_reply_to_id: Option<String>,
    pub reblog: Option<Box<Status>>,
}

#[derive(Deserialize)]
pub(super) struct Context {
    #[serde(default)]
    pub ancestors: Vec<Status>,
    #[serde(default)]
    pub descendants: Vec<Status>,
}

#[derive(Deserialize, Default)]
pub(super) struct Relationship {
    #[serde(default)]
    pub following: bool,
}

#[derive(Deserialize)]
pub(super) struct Tag {
    pub name: String,
}

#[derive(Deserialize)]
pub(super) struct List {
    pub id: String,
    pub title: String,
}

#[derive(Deserialize)]
pub(super) struct Notification {
    pub id: String,
    #[serde(rename = "type")]
    pub notification_type: String,
    pub created_at: String,
    pub account: Account,
    pub status: Option<Status>,
}

#[derive(Deserialize)]
pub(super) struct TrendHistory {
    pub uses: String,
}

#[derive(Deserialize)]
pub(super) struct TrendTag {
    pub name: String,
    #[serde(default)]
    pub history: Vec<TrendHistory>,
}

#[derive(Deserialize)]
pub(super) struct Suggestion {
    pub account: Account,
}
