pub mod bluesky;
pub mod mastodon;
pub mod social;
use crate::{
    error::AppError,
    models::{PlatformCapabilities, PreparedPost, PublishedPost},
};
use async_trait::async_trait;
#[async_trait]
pub trait SocialProvider: Send + Sync {
    async fn home_feed(&self, _cursor: Option<&str>) -> Result<social::FeedPage, AppError> {
        Err(AppError::Provider(
            "Home feed is unavailable for this provider".into(),
        ))
    }
    async fn own_feed(
        &self,
        _kind: social::ProfileFeedKind,
        _cursor: Option<&str>,
    ) -> Result<social::FeedPage, AppError> {
        Err(AppError::Provider(
            "Own profile feed is unavailable for this provider".into(),
        ))
    }
    async fn own_profile(&self) -> Result<social::ProfileDetails, AppError> {
        Err(AppError::Provider(
            "Own profile is unavailable for this provider".into(),
        ))
    }
    async fn profile(&self, _profile_id: &str) -> Result<social::ProfileDetails, AppError> {
        Err(AppError::Provider(
            "Profile details are unavailable for this provider".into(),
        ))
    }
    async fn profile_feed(
        &self,
        _profile_id: &str,
        _kind: social::ProfileFeedKind,
        _cursor: Option<&str>,
    ) -> Result<social::FeedPage, AppError> {
        Err(AppError::Provider(
            "Profile feed is unavailable for this provider".into(),
        ))
    }
    async fn thread(&self, _post_id: &str) -> Result<social::ThreadView, AppError> {
        Err(AppError::Provider(
            "Thread is unavailable for this provider".into(),
        ))
    }
    async fn tag_feed(
        &self,
        _tag: &str,
        _cursor: Option<&str>,
    ) -> Result<social::FeedPage, AppError> {
        Err(AppError::Provider(
            "Tag feed is unavailable for this provider".into(),
        ))
    }
    async fn followed_sources_page(
        &self,
        _cursor: Option<&str>,
    ) -> Result<social::SourcePage, AppError> {
        Err(AppError::Provider(
            "Followed sources are unavailable for this provider".into(),
        ))
    }
    async fn source_feed(
        &self,
        _source: &social::FollowedSource,
        _cursor: Option<&str>,
    ) -> Result<social::FeedPage, AppError> {
        Err(AppError::Provider(
            "Source feed is unavailable for this provider".into(),
        ))
    }
    async fn notifications(
        &self,
        _cursor: Option<&str>,
    ) -> Result<social::NotificationPage, AppError> {
        Err(AppError::Provider(
            "Notifications are unavailable for this provider".into(),
        ))
    }
    async fn mark_notifications_read(&self, _ids: &[String]) -> Result<(), AppError> {
        Err(AppError::Provider(
            "Notification read state is unavailable for this provider".into(),
        ))
    }
    async fn social_action(
        &self,
        _action: social::SocialAction,
    ) -> Result<social::SocialActionResult, AppError> {
        Err(AppError::Provider(
            "This social action is unavailable for this provider".into(),
        ))
    }
    async fn timeline(
        &self,
        _account_id: &str,
        _account_handle: &str,
        _cursor: Option<&str>,
    ) -> Result<serde_json::Value, AppError> {
        Err(AppError::Provider(
            "Home timeline is unavailable for this provider".into(),
        ))
    }
    async fn discovery(
        &self,
        _account_id: &str,
        _account_handle: &str,
    ) -> Result<serde_json::Value, AppError> {
        Err(AppError::Provider(
            "Discovery is unavailable for this provider".into(),
        ))
    }
    async fn following_sources(&self, _account_id: &str) -> Result<serde_json::Value, AppError> {
        Err(AppError::Provider(
            "Followed sources are unavailable for this provider".into(),
        ))
    }
    async fn hashtags(
        &self,
        _query: &str,
    ) -> Result<Vec<crate::hashtags::HashtagSuggestion>, AppError> {
        Err(AppError::Provider(
            "Hashtag lookup is unavailable for this provider".into(),
        ))
    }
    async fn capabilities(&self) -> Result<PlatformCapabilities, AppError>;
    async fn publish(&self, post: PreparedPost) -> Result<PublishedPost, AppError>;
    async fn reply(
        &self,
        parent: &PublishedPost,
        post: PreparedPost,
    ) -> Result<PublishedPost, AppError>;
}

pub fn safe_error_body(body: &str) -> String {
    const LIMIT: usize = 500;
    let sanitized: String = body
        .chars()
        .map(|character| {
            if character.is_control() {
                ' '
            } else {
                character
            }
        })
        .take(LIMIT)
        .collect();
    if sanitized.is_empty() {
        "empty response".into()
    } else {
        sanitized
    }
}

pub(crate) fn legacy_sources(
    account_id: &str,
    sources: Vec<social::FollowedSource>,
) -> serde_json::Value {
    serde_json::Value::Array(
        sources
            .into_iter()
            .map(|source| {
                let source_type = match source.source_type {
                    social::SourceKind::Person => "PERSON",
                    social::SourceKind::Tag => "TOPIC",
                    social::SourceKind::List | social::SourceKind::Feed => "FEED",
                };
                serde_json::json!({
                    "id": source.id,
                    "provider": source.provider,
                    "type": source_type,
                    "title": source.title,
                    "description": source.description,
                    "accountId": account_id,
                    "remoteId": source.remote_id,
                })
            })
            .collect(),
    )
}

pub(crate) fn legacy_post(
    post: social::SocialPost,
    account_id: &str,
    account_handle: &str,
) -> serde_json::Value {
    let provider = post.provider;
    let media = post
        .media
        .into_iter()
        .map(|item| {
            serde_json::json!({
                "url": item.url,
                "alt": item.alt,
                "type": item.media_type,
            })
        })
        .collect::<Vec<_>>();
    serde_json::json!({
        "canonicalKey": post.canonical_key,
        "provider": provider,
        "remoteId": post.remote_id,
        "remoteUrl": post.remote_url,
        "author": post.author,
        "text": post.text,
        "createdAt": post.created_at,
        "media": media,
        "sources": [{
            "accountId": account_id,
            "accountHandle": account_handle,
            "provider": provider,
        }],
        "metrics": post.metrics,
        "viewer": post.viewer,
        "capabilities": {
            "openOriginal": true,
            "reply": true,
            "like": true,
            "repost": true,
        },
    })
}

pub fn now_iso8601() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    // ATProto accepts RFC 3339 timestamps. Avoid another time dependency by formatting UTC here.
    let days = seconds / 86_400;
    let second_of_day = seconds % 86_400;
    let (year, month, day) = civil_from_days(days as i64);
    format!(
        "{year:04}-{month:02}-{day:02}T{:02}:{:02}:{:02}Z",
        second_of_day / 3600,
        (second_of_day % 3600) / 60,
        second_of_day % 60
    )
}

fn civil_from_days(days_since_epoch: i64) -> (i64, i64, i64) {
    let z = days_since_epoch + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let day_of_era = z - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    (year, month, day)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn error_bodies_are_bounded_and_sanitized() {
        let body = format!("secret\n{}", "x".repeat(600));
        let safe = safe_error_body(&body);
        assert!(!safe.contains('\n'));
        assert_eq!(safe.chars().count(), 500);
    }

    #[test]
    fn timestamps_are_rfc3339_utc() {
        let timestamp = now_iso8601();
        assert_eq!(timestamp.len(), 20);
        assert_eq!(&timestamp[4..5], "-");
        assert!(timestamp.ends_with('Z'));
    }
}

#[cfg(test)]
mod media_tests;

#[cfg(test)]
mod bluesky_notification_tests;
#[cfg(test)]
mod bluesky_session_tests;
#[cfg(test)]
mod discovery_test;
#[cfg(test)]
mod hashtag_tests;
#[cfg(test)]
mod social_test;
#[cfg(test)]
mod test_http;
