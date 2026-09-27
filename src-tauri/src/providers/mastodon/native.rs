//! Mastodon API response shapes used before provider-neutral normalization.
//!
//! Statuses, relationships, lists, notifications, and trends are deserialized
//! only to the fields needed by the provider's read and discovery operations.

use serde::Deserialize;

/// Remote account identity, profile text, and optional activity counts.
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

/// Media URL, description, and Mastodon attachment type.
#[derive(Clone, Deserialize)]
pub(super) struct Attachment {
    pub url: String,
    pub description: Option<String>,
    #[serde(rename = "type")]
    pub media_type: String,
}

/// Mastodon status with engagement and optional nested reblog data.
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

/// Ancestors and descendants of a requested status.
#[derive(Deserialize)]
pub(super) struct Context {
    #[serde(default)]
    pub ancestors: Vec<Status>,
    #[serde(default)]
    pub descendants: Vec<Status>,
}

/// Whether the connected account follows the requested account.
#[derive(Deserialize, Default)]
pub(super) struct Relationship {
    #[serde(default)]
    pub following: bool,
}

/// A followed hashtag.
#[derive(Deserialize)]
pub(super) struct Tag {
    pub name: String,
}

/// A Mastodon list identifier and title.
#[derive(Deserialize)]
pub(super) struct List {
    pub id: String,
    pub title: String,
}

/// Inbox activity with an optional related status.
#[derive(Deserialize)]
pub(super) struct Notification {
    pub id: String,
    #[serde(rename = "type")]
    pub notification_type: String,
    pub created_at: String,
    pub account: Account,
    pub status: Option<Status>,
}

/// One trend interval's usage count, encoded as a string by Mastodon.
#[derive(Deserialize)]
pub(super) struct TrendHistory {
    pub uses: String,
}

/// Trending hashtag and its reported usage history.
#[derive(Deserialize)]
pub(super) struct TrendTag {
    pub name: String,
    #[serde(default)]
    pub history: Vec<TrendHistory>,
}

/// Suggested account wrapper returned by the discovery endpoint.
#[derive(Deserialize)]
pub(super) struct Suggestion {
    pub account: Account,
}
