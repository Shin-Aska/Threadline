//! Bluesky social-source and notification normalization.
//!
//! Notification subject enrichment deduplicates URIs and fetches them in batches of 25. Failed
//! enrichment leaves the affected related post absent while preserving the inbox item itself.

use crate::{
    error::AppError,
    models::ProviderKind,
    providers::social::{
        FeedPage, FollowedSource, NotificationItem, NotificationKind, NotificationPage, SourceKind,
        SourcePage,
    },
};
use std::collections::{HashMap, HashSet};

use super::{native, normalize, BlueskyProvider};

impl BlueskyProvider {
    pub(super) async fn followed_sources_page_impl(
        &self,
        cursor: Option<&str>,
    ) -> Result<SourcePage, AppError> {
        let did = self.account_did().await?;
        let mut follows_query = vec![("actor", did.as_str()), ("limit", "100")];
        if let Some(cursor) = cursor {
            follows_query.push(("cursor", cursor));
        }
        let follows: native::ActorsResponse = self
            .social_get("app.bsky.graph.getFollows", &follows_query)
            .await?;
        let (lists, preferences) = if cursor.is_none() {
            let lists = self
                .social_get(
                    "app.bsky.graph.getLists",
                    &[("actor", did.as_str()), ("limit", "100")],
                )
                .await?;
            let preferences = self
                .social_get("app.bsky.actor.getPreferences", &[])
                .await?;
            (lists, preferences)
        } else {
            (
                native::ListsResponse { lists: Vec::new() },
                native::PreferencesResponse {
                    preferences: Vec::new(),
                },
            )
        };
        let mut sources = follows
            .follows
            .into_iter()
            .map(|actor| FollowedSource {
                id: format!("BLUESKY:person:{}", actor.did),
                provider: ProviderKind::Bluesky,
                source_type: SourceKind::Person,
                title: actor
                    .display_name
                    .clone()
                    .unwrap_or_else(|| actor.handle.clone()),
                description: Some(format!("@{}", actor.handle)),
                remote_id: actor.did,
            })
            .collect::<Vec<_>>();
        sources.extend(lists.lists.into_iter().map(|list| FollowedSource {
            id: format!("BLUESKY:list:{}", list.uri),
            provider: ProviderKind::Bluesky,
            source_type: SourceKind::List,
            title: list.name,
            description: list.description,
            remote_id: list.uri,
        }));
        sources.extend(
            preferences
                .preferences
                .into_iter()
                .filter(|preference| {
                    preference.record_type == "app.bsky.actor.defs#savedFeedsPrefV2"
                })
                .flat_map(|preference| preference.items.unwrap_or_default())
                .filter(|item| item.item_type == "feed")
                .map(|item| FollowedSource {
                    id: format!("BLUESKY:feed:{}", item.value),
                    provider: ProviderKind::Bluesky,
                    source_type: SourceKind::Feed,
                    title: item.value.rsplit('/').next().unwrap_or("Feed").to_owned(),
                    description: item.pinned.then(|| "Pinned Bluesky feed".into()),
                    remote_id: item.value,
                }),
        );
        Ok(SourcePage {
            sources,
            cursor: follows.cursor,
        })
    }

    pub(super) async fn source_feed_page(
        &self,
        source: &FollowedSource,
        cursor: Option<&str>,
    ) -> Result<FeedPage, AppError> {
        match source.source_type {
            SourceKind::Person => {
                self.profile_feed_page(
                    &source.remote_id,
                    crate::providers::social::ProfileFeedKind::Posts,
                    cursor,
                )
                .await
            }
            SourceKind::Tag => self.tag_feed_page(&source.remote_id, cursor).await,
            SourceKind::List => {
                let mut query = vec![("list", source.remote_id.as_str()), ("limit", "50")];
                if let Some(cursor) = cursor {
                    query.push(("cursor", cursor));
                }
                let response: native::FeedResponse =
                    self.social_get("app.bsky.feed.getListFeed", &query).await?;
                Ok(FeedPage {
                    posts: response
                        .feed
                        .into_iter()
                        .map(|item| normalize::post(item.post))
                        .collect(),
                    cursor: response.cursor,
                })
            }
            SourceKind::Feed => {
                let mut query = vec![("feed", source.remote_id.as_str()), ("limit", "50")];
                if let Some(cursor) = cursor {
                    query.push(("cursor", cursor));
                }
                let response: native::FeedResponse =
                    self.social_get("app.bsky.feed.getFeed", &query).await?;
                Ok(FeedPage {
                    posts: response
                        .feed
                        .into_iter()
                        .map(|item| normalize::post(item.post))
                        .collect(),
                    cursor: response.cursor,
                })
            }
        }
    }

    pub(super) async fn notification_page(
        &self,
        cursor: Option<&str>,
    ) -> Result<NotificationPage, AppError> {
        let mut query = vec![("limit", "50")];
        if let Some(cursor) = cursor {
            query.push(("cursor", cursor));
        }
        let response: native::NotificationResponse = self
            .social_get("app.bsky.notification.listNotifications", &query)
            .await?;
        let mut seen_subjects = HashSet::new();
        let subjects = response
            .notifications
            .iter()
            .filter(|value| !matches!(value.reason.as_str(), "mention" | "reply"))
            .filter_map(|value| value.reason_subject.as_deref())
            .filter(|subject| seen_subjects.insert(*subject))
            .collect::<Vec<_>>();
        let mut related_posts = HashMap::with_capacity(subjects.len());
        for subjects in subjects.chunks(25) {
            let query = subjects
                .iter()
                .map(|subject| ("uris", *subject))
                .collect::<Vec<_>>();
            if let Ok(response) = self
                .social_get::<native::PostsResponse>("app.bsky.feed.getPosts", &query)
                .await
            {
                related_posts.extend(
                    response
                        .posts
                        .into_iter()
                        .map(|post| (post.uri.clone(), post)),
                );
            }
        }
        let mut notifications = Vec::with_capacity(response.notifications.len());
        for value in response.notifications {
            let kind = match value.reason.as_str() {
                "mention" => NotificationKind::Mention,
                "reply" => NotificationKind::Reply,
                "like" => NotificationKind::Like,
                "repost" => NotificationKind::Repost,
                "follow" => NotificationKind::Follow,
                "quote" => NotificationKind::Quote,
                _ => NotificationKind::Other,
            };
            let embedded_post = matches!(kind, NotificationKind::Mention | NotificationKind::Reply)
                .then(|| {
                    normalize::post(native::PostView {
                        uri: value.uri.clone(),
                        cid: value.cid,
                        author: value.author.clone(),
                        record: value.record,
                        embed: None,
                        reply_count: None,
                        repost_count: None,
                        like_count: None,
                        viewer: None,
                    })
                });
            let post = match (embedded_post, value.reason_subject.as_deref()) {
                (Some(post), _) => Some(post),
                (None, Some(subject)) => related_posts.get(subject).cloned().map(normalize::post),
                (None, None) => None,
            };
            notifications.push(NotificationItem {
                id: value.uri,
                kind,
                created_at: value.indexed_at,
                actor: normalize::actor(value.author),
                post,
                unread: !value.is_read,
            });
        }
        Ok(NotificationPage {
            notifications,
            cursor: response.cursor,
        })
    }

    pub(super) async fn mark_read(&self, ids: &[String]) -> Result<(), AppError> {
        if ids.is_empty() {
            return Ok(());
        }
        let _: serde_json::Value = self
            .post_json(
                "app.bsky.notification.updateSeen",
                serde_json::json!({"seenAt": crate::providers::now_iso8601()}),
            )
            .await?;
        Ok(())
    }
}
