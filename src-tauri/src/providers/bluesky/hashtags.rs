use super::BlueskyProvider;
use crate::{
    error::AppError,
    hashtags::{HashtagActivity, HashtagSuggestion},
};
use serde::Deserialize;
use std::time::Duration;
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Search {
    hits_total: Option<u64>,
}
impl BlueskyProvider {
    pub(super) async fn search_hashtags(
        &self,
        query: &str,
    ) -> Result<Vec<HashtagSuggestion>, AppError> {
        if query.is_empty() {
            return Ok(Vec::new());
        }
        let term = format!("#{query}");
        let result: Search = tokio::time::timeout(
            Duration::from_secs(12),
            self.get_json_with_timeout(
                "app.bsky.feed.searchPosts",
                &[("q", term.as_str()), ("tag", query), ("limit", "1")],
                Duration::from_secs(12),
            ),
        )
        .await
        .map_err(|_| AppError::Provider("Bluesky hashtag lookup timed out".into()))??;
        Ok(vec![HashtagSuggestion {
            name: query.into(),
            activity: HashtagActivity::Bluesky {
                matches: result.hits_total,
            },
        }])
    }
}
