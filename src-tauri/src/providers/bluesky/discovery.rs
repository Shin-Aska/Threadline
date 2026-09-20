use crate::error::AppError;

use super::{native, normalize, BlueskyProvider};

impl BlueskyProvider {
    pub(super) async fn discovery_result(
        &self,
        account_id: &str,
        account_handle: &str,
    ) -> Result<serde_json::Value, AppError> {
        let trends: native::TrendingTopicsResponse = self
            .get_json("app.bsky.unspecced.getTrendingTopics", &[("limit", "20")])
            .await?;
        let suggestions: native::SuggestionsResponse = self
            .get_json("app.bsky.actor.getSuggestions", &[("limit", "20")])
            .await?;
        let topics = trends
            .topics
            .into_iter()
            .chain(trends.suggested)
            .map(|topic| {
                let name = topic.display_name.unwrap_or_else(|| topic.topic.clone());
                serde_json::json!({
                    "key": topic.topic.to_lowercase(),
                    "name": name,
                    "sources": [{
                        "accountId": account_id,
                        "accountHandle": account_handle,
                        "provider": "BLUESKY",
                    }],
                })
            })
            .collect::<Vec<_>>();
        let suggested_accounts = suggestions
            .actors
            .into_iter()
            .map(normalize::actor)
            .collect::<Vec<_>>();
        Ok(serde_json::json!({
            "topics": topics,
            "suggestedAccounts": suggested_accounts,
            "popularPosts": [],
        }))
    }
}
