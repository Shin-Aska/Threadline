use crate::error::AppError;

use super::{native, normalize, MastodonProvider};

impl MastodonProvider {
    pub(super) async fn discovery_result(
        &self,
        account_id: &str,
        account_handle: &str,
    ) -> Result<serde_json::Value, AppError> {
        let tags: Vec<native::TrendTag> = self
            .social_get(
                self.client
                    .get(format!("{}/api/v1/trends/tags", self.base_url))
                    .query(&[("limit", "20")]),
                "trending tags",
            )
            .await?;
        let suggestions: Vec<native::Suggestion> = self
            .social_get(
                self.client
                    .get(format!("{}/api/v2/suggestions", self.base_url))
                    .query(&[("limit", "20")]),
                "account suggestions",
            )
            .await?;
        let statuses: Vec<native::Status> = self
            .social_get(
                self.client
                    .get(format!("{}/api/v1/trends/statuses", self.base_url))
                    .query(&[("limit", "20")]),
                "trending statuses",
            )
            .await?;
        let topics = tags
            .into_iter()
            .map(|tag| {
                let history = tag
                    .history
                    .into_iter()
                    .filter_map(|entry| entry.uses.parse::<u64>().ok())
                    .collect::<Vec<_>>();
                serde_json::json!({
                    "key": tag.name.to_lowercase(),
                    "name": tag.name,
                    "sources": [source(account_id, account_handle)],
                    "postCount": history.iter().sum::<u64>(),
                    "history": history,
                })
            })
            .collect::<Vec<_>>();
        let suggested_accounts = suggestions
            .into_iter()
            .map(|value| normalize::actor(value.account))
            .collect::<Vec<_>>();
        let popular_posts = statuses
            .into_iter()
            .map(|value| normalize::post(&self.base_url, value))
            .collect::<Result<Vec<_>, _>>()?
            .into_iter()
            .map(|post| crate::providers::legacy_post(post, account_id, account_handle))
            .collect::<Vec<_>>();
        Ok(serde_json::json!({
            "topics": topics,
            "suggestedAccounts": suggested_accounts,
            "popularPosts": popular_posts,
        }))
    }
}

fn source(account_id: &str, account_handle: &str) -> serde_json::Value {
    serde_json::json!({
        "accountId": account_id,
        "accountHandle": account_handle,
        "provider": "MASTODON",
    })
}
