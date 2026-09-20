use crate::{
    error::AppError,
    providers::social::{FeedPage, ProfileDetails, ProfileFeedKind, ThreadView},
};

use super::{native, normalize, MastodonProvider};

impl MastodonProvider {
    pub(super) async fn home_feed_page(&self, cursor: Option<&str>) -> Result<FeedPage, AppError> {
        let mut request = self
            .client
            .get(format!("{}/api/v1/timelines/home", self.base_url))
            .query(&[("limit", "40")]);
        if let Some(cursor) = cursor {
            request = request.query(&[("max_id", cursor)]);
        }
        let statuses = self.social_get(request, "home feed").await?;
        self.status_page(statuses)
    }

    pub(super) async fn social_get<T: serde::de::DeserializeOwned>(
        &self,
        request: reqwest::RequestBuilder,
        label: &str,
    ) -> Result<T, AppError> {
        let response = request
            .bearer_auth(&self.access_token)
            .timeout(std::time::Duration::from_secs(60))
            .send()
            .await
            .map_err(|error| AppError::Provider(format!("Mastodon {label} failed: {error}")))?;
        let status = response.status();
        let body = response.text().await.map_err(|error| {
            AppError::Provider(format!(
                "Mastodon {label} response could not be read: {error}"
            ))
        })?;
        if !status.is_success() {
            return Err(AppError::Provider(format!(
                "Mastodon {label} returned {status}: {}",
                crate::providers::safe_error_body(&body)
            )));
        }
        serde_json::from_str(&body)
            .map_err(|error| AppError::Provider(format!("Invalid Mastodon {label}: {error}")))
    }

    pub(super) async fn profile_details(
        &self,
        profile_id: &str,
    ) -> Result<ProfileDetails, AppError> {
        let account: native::Account = self
            .social_get(
                self.client
                    .get(format!("{}/api/v1/accounts/{profile_id}", self.base_url)),
                "profile",
            )
            .await?;
        let relationships: Vec<native::Relationship> = self
            .social_get(
                self.client
                    .get(format!("{}/api/v1/accounts/relationships", self.base_url))
                    .query(&[("id[]", profile_id)]),
                "relationship",
            )
            .await?;
        let followed_by_me = relationships.first().is_some_and(|value| value.following);
        let description = regex::Regex::new("<[^>]+>")
            .map_err(|error| AppError::Provider(error.to_string()))?
            .replace_all(&account.note, "")
            .to_string();
        let followers_count = account.followers_count;
        let following_count = account.following_count;
        let posts_count = account.statuses_count;
        Ok(ProfileDetails {
            actor: normalize::actor(account),
            description,
            followers_count,
            following_count,
            posts_count,
            followed_by_me,
            follow_uri: None,
        })
    }

    pub(super) async fn profile_feed_page(
        &self,
        profile_id: &str,
        kind: ProfileFeedKind,
        cursor: Option<&str>,
    ) -> Result<FeedPage, AppError> {
        let mut request = self
            .client
            .get(format!(
                "{}/api/v1/accounts/{profile_id}/statuses",
                self.base_url
            ))
            .query(&[("limit", "40"), ("exclude_reblogs", "true")]);
        request = match kind {
            ProfileFeedKind::Posts => request.query(&[("exclude_replies", "true")]),
            ProfileFeedKind::Replies => request,
            ProfileFeedKind::Media => request.query(&[("only_media", "true")]),
        };
        if let Some(cursor) = cursor {
            request = request.query(&[("max_id", cursor)]);
        }
        let statuses: Vec<native::Status> = self.social_get(request, "profile feed").await?;
        let statuses = match kind {
            ProfileFeedKind::Replies => statuses
                .into_iter()
                .filter(|status| status.in_reply_to_id.is_some())
                .collect(),
            ProfileFeedKind::Posts | ProfileFeedKind::Media => statuses,
        };
        self.status_page(statuses)
    }

    pub(super) async fn own_feed_page(
        &self,
        kind: ProfileFeedKind,
        cursor: Option<&str>,
    ) -> Result<FeedPage, AppError> {
        let account: native::Account = self
            .social_get(
                self.client.get(format!(
                    "{}/api/v1/accounts/verify_credentials",
                    self.base_url
                )),
                "own profile",
            )
            .await?;
        self.profile_feed_page(&account.id, kind, cursor).await
    }

    pub(super) async fn own_profile_details(&self) -> Result<ProfileDetails, AppError> {
        let account: native::Account = self
            .social_get(
                self.client.get(format!(
                    "{}/api/v1/accounts/verify_credentials",
                    self.base_url
                )),
                "own profile",
            )
            .await?;
        self.profile_details(&account.id).await
    }

    pub(super) fn status_page(&self, statuses: Vec<native::Status>) -> Result<FeedPage, AppError> {
        let cursor = statuses.last().map(|status| status.id.clone());
        let posts = statuses
            .into_iter()
            .map(|status| normalize::post(&self.base_url, status))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(FeedPage { posts, cursor })
    }

    pub(super) async fn thread_view(&self, post_id: &str) -> Result<ThreadView, AppError> {
        let post: native::Status = self
            .social_get(
                self.client
                    .get(format!("{}/api/v1/statuses/{post_id}", self.base_url)),
                "status",
            )
            .await?;
        let context: native::Context = self
            .social_get(
                self.client.get(format!(
                    "{}/api/v1/statuses/{post_id}/context",
                    self.base_url
                )),
                "thread",
            )
            .await?;
        Ok(ThreadView {
            ancestors: context
                .ancestors
                .into_iter()
                .map(|value| normalize::post(&self.base_url, value))
                .collect::<Result<_, _>>()?,
            post: normalize::post(&self.base_url, post)?,
            replies: context
                .descendants
                .into_iter()
                .map(|value| normalize::post(&self.base_url, value))
                .collect::<Result<_, _>>()?,
            cursor: None,
        })
    }

    pub(super) async fn tag_feed_page(
        &self,
        tag: &str,
        cursor: Option<&str>,
    ) -> Result<FeedPage, AppError> {
        if tag.is_empty()
            || !tag
                .chars()
                .all(|character| character.is_alphanumeric() || character == '_')
        {
            return Err(AppError::Validation(
                "Tag contains unsupported characters".into(),
            ));
        }
        let mut request = self
            .client
            .get(format!("{}/api/v1/timelines/tag/{tag}", self.base_url))
            .query(&[("limit", "40")]);
        if let Some(cursor) = cursor {
            request = request.query(&[("max_id", cursor)]);
        }
        let statuses = self.social_get(request, "tag feed").await?;
        self.status_page(statuses)
    }
}
