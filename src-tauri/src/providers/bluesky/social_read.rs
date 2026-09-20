use crate::{
    error::AppError,
    providers::social::{FeedPage, ProfileDetails, ProfileFeedKind, ThreadView},
};

use super::{native, normalize, BlueskyProvider};

impl BlueskyProvider {
    pub(super) async fn home_feed_page(&self, cursor: Option<&str>) -> Result<FeedPage, AppError> {
        let mut query = vec![("limit", "50")];
        if let Some(cursor) = cursor {
            query.push(("cursor", cursor));
        }
        let response: native::FeedResponse =
            self.social_get("app.bsky.feed.getTimeline", &query).await?;
        Ok(FeedPage {
            posts: response
                .feed
                .into_iter()
                .map(|item| normalize::post(item.post))
                .collect(),
            cursor: response.cursor,
        })
    }

    pub(super) async fn social_get<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        query: &[(&str, &str)],
    ) -> Result<T, AppError> {
        self.get_json(path, query).await
    }

    pub(super) async fn profile_details(
        &self,
        profile_id: &str,
    ) -> Result<ProfileDetails, AppError> {
        let value: native::Actor = self
            .social_get("app.bsky.actor.getProfile", &[("actor", profile_id)])
            .await?;
        let description = value.description.clone().unwrap_or_default();
        let followers_count = value.followers_count;
        let following_count = value.follows_count;
        let posts_count = value.posts_count;
        let follow_uri = value
            .viewer
            .as_ref()
            .and_then(|viewer| viewer.following.clone());
        Ok(ProfileDetails {
            actor: normalize::actor(value),
            description,
            followers_count,
            following_count,
            posts_count,
            followed_by_me: follow_uri.is_some(),
            follow_uri,
        })
    }

    pub(super) async fn profile_feed_page(
        &self,
        profile_id: &str,
        kind: ProfileFeedKind,
        cursor: Option<&str>,
    ) -> Result<FeedPage, AppError> {
        let filter = match kind {
            ProfileFeedKind::Posts => "posts_no_replies",
            ProfileFeedKind::Replies => "posts_with_replies",
            ProfileFeedKind::Media => "posts_with_media",
        };
        let mut query = vec![("actor", profile_id), ("filter", filter), ("limit", "50")];
        if let Some(cursor) = cursor {
            query.push(("cursor", cursor));
        }
        let response: native::FeedResponse = self
            .social_get("app.bsky.feed.getAuthorFeed", &query)
            .await?;
        let mut posts = response
            .feed
            .into_iter()
            .map(|item| normalize::post(item.post))
            .collect::<Vec<_>>();
        if matches!(kind, ProfileFeedKind::Replies) {
            posts.retain(|post| post.reply_parent_id.is_some());
        }
        Ok(FeedPage {
            posts,
            cursor: response.cursor,
        })
    }

    pub(super) async fn own_feed_page(
        &self,
        kind: ProfileFeedKind,
        cursor: Option<&str>,
    ) -> Result<FeedPage, AppError> {
        let did = self.account_did().await?;
        self.profile_feed_page(&did, kind, cursor).await
    }

    pub(super) async fn own_profile_details(&self) -> Result<ProfileDetails, AppError> {
        let did = self.account_did().await?;
        self.profile_details(&did).await
    }

    pub(super) async fn thread_view(&self, post_id: &str) -> Result<ThreadView, AppError> {
        let response: native::ThreadResponse = self
            .social_get(
                "app.bsky.feed.getPostThread",
                &[("uri", post_id), ("depth", "100"), ("parentHeight", "100")],
            )
            .await?;
        let post = response
            .thread
            .post
            .ok_or_else(|| AppError::Provider("Bluesky thread root is unavailable".into()))?;
        let mut ancestors = Vec::new();
        let mut parent = response.thread.parent;
        while let Some(entry) = parent {
            if let Some(post) = entry.post {
                ancestors.push(normalize::post(post));
            }
            parent = entry.parent;
        }
        ancestors.reverse();
        let replies = response
            .thread
            .replies
            .into_iter()
            .filter_map(|entry| entry.post)
            .map(normalize::post)
            .collect();
        Ok(ThreadView {
            ancestors,
            post: normalize::post(post),
            replies,
            cursor: None,
        })
    }

    pub(super) async fn tag_feed_page(
        &self,
        tag: &str,
        cursor: Option<&str>,
    ) -> Result<FeedPage, AppError> {
        let query_text = format!("#{tag}");
        let mut query = vec![("q", query_text.as_str()), ("limit", "50")];
        if let Some(cursor) = cursor {
            query.push(("cursor", cursor));
        }
        let response: native::PostsResponse =
            self.social_get("app.bsky.feed.searchPosts", &query).await?;
        Ok(FeedPage {
            posts: response.posts.into_iter().map(normalize::post).collect(),
            cursor: response.cursor,
        })
    }
}
