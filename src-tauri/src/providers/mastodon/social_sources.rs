use crate::{
    error::AppError,
    models::ProviderKind,
    providers::social::{
        FeedPage, FollowedSource, NotificationItem, NotificationKind, NotificationPage, SourceKind,
        SourcePage,
    },
};

use super::{native, normalize, MastodonProvider};

impl MastodonProvider {
    pub(super) async fn followed_sources_page_impl(
        &self,
        cursor: Option<&str>,
    ) -> Result<SourcePage, AppError> {
        let own: native::Account = self
            .social_get(
                self.client.get(format!(
                    "{}/api/v1/accounts/verify_credentials",
                    self.base_url
                )),
                "own profile",
            )
            .await?;
        let mut people_request = self
            .client
            .get(format!(
                "{}/api/v1/accounts/{}/following",
                self.base_url, own.id
            ))
            .query(&[("limit", "80")]);
        if let Some(cursor) = cursor {
            people_request = people_request.query(&[("max_id", cursor)]);
        }
        let people: Vec<native::Account> = self.social_get(people_request, "following").await?;
        let (tags, lists) = if cursor.is_none() {
            let tags: Vec<native::Tag> = self
                .social_get(
                    self.client
                        .get(format!("{}/api/v1/followed_tags", self.base_url)),
                    "followed tags",
                )
                .await?;
            let lists: Vec<native::List> = self
                .social_get(
                    self.client.get(format!("{}/api/v1/lists", self.base_url)),
                    "lists",
                )
                .await?;
            (tags, lists)
        } else {
            (Vec::new(), Vec::new())
        };
        let cursor = people.last().map(|account| account.id.clone());
        let mut sources = people
            .into_iter()
            .map(|account| FollowedSource {
                id: format!("MASTODON:{}:person:{}", self.base_url, account.id),
                provider: ProviderKind::Mastodon,
                source_type: SourceKind::Person,
                title: if account.display_name.is_empty() {
                    account.acct.clone()
                } else {
                    account.display_name
                },
                description: Some(format!("@{}", account.acct)),
                remote_id: account.id,
            })
            .collect::<Vec<_>>();
        sources.extend(tags.into_iter().map(|tag| FollowedSource {
            id: format!("MASTODON:{}:tag:{}", self.base_url, tag.name.to_lowercase()),
            provider: ProviderKind::Mastodon,
            source_type: SourceKind::Tag,
            title: format!("#{}", tag.name),
            description: None,
            remote_id: tag.name,
        }));
        sources.extend(lists.into_iter().map(|list| FollowedSource {
            id: format!("MASTODON:{}:list:{}", self.base_url, list.id),
            provider: ProviderKind::Mastodon,
            source_type: SourceKind::List,
            title: list.title,
            description: None,
            remote_id: list.id,
        }));
        Ok(SourcePage { sources, cursor })
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
                let mut request = self
                    .client
                    .get(format!(
                        "{}/api/v1/timelines/list/{}",
                        self.base_url, source.remote_id
                    ))
                    .query(&[("limit", "40")]);
                if let Some(cursor) = cursor {
                    request = request.query(&[("max_id", cursor)]);
                }
                let statuses = self.social_get(request, "list feed").await?;
                self.status_page(statuses)
            }
            SourceKind::Feed => Err(AppError::Provider(
                "Mastodon does not expose custom feed sources".into(),
            )),
        }
    }

    pub(super) async fn notification_page(
        &self,
        cursor: Option<&str>,
    ) -> Result<NotificationPage, AppError> {
        let mut request = self
            .client
            .get(format!("{}/api/v1/notifications", self.base_url))
            .query(&[("limit", "40")]);
        if let Some(cursor) = cursor {
            request = request.query(&[("max_id", cursor)]);
        }
        let values: Vec<native::Notification> = self.social_get(request, "notifications").await?;
        let cursor = values.last().map(|value| value.id.clone());
        let notifications = values
            .into_iter()
            .map(|value| {
                let kind = match value.notification_type.as_str() {
                    "mention" => NotificationKind::Mention,
                    "favourite" => NotificationKind::Like,
                    "reblog" => NotificationKind::Repost,
                    "follow" | "follow_request" => NotificationKind::Follow,
                    "status" | "poll" | "update" => NotificationKind::Other,
                    _ => NotificationKind::Other,
                };
                Ok(NotificationItem {
                    id: value.id,
                    kind,
                    created_at: value.created_at,
                    actor: normalize::actor(value.account),
                    post: value
                        .status
                        .map(|status| normalize::post(&self.base_url, status))
                        .transpose()?,
                    unread: true,
                })
            })
            .collect::<Result<Vec<_>, AppError>>()?;
        Ok(NotificationPage {
            notifications,
            cursor,
        })
    }
}
