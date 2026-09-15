use super::MastodonProvider;
use crate::{
    error::AppError,
    hashtags::{response, valid_query, HashtagActivity, HashtagSuggestion},
};
use serde::Deserialize;
#[derive(Deserialize)]
struct Tag {
    name: String,
    history: Option<Vec<Day>>,
}
#[derive(Deserialize)]
struct Day {
    uses: String,
}
#[derive(Deserialize)]
struct Search {
    hashtags: Vec<Tag>,
}
impl MastodonProvider {
    pub(super) async fn search_hashtags(
        &self,
        query: &str,
    ) -> Result<Vec<HashtagSuggestion>, AppError> {
        let base = self.base_url.trim_end_matches('/');
        let tags: Vec<Tag> = if query.is_empty() {
            response(
                self.client
                    .get(format!("{base}/api/v1/trends/tags"))
                    .bearer_auth(&self.access_token)
                    .query(&[("limit", "20")]),
            )
            .await?
        } else {
            let found: Search = response(
                self.client
                    .get(format!("{base}/api/v2/search"))
                    .bearer_auth(&self.access_token)
                    .query(&[("q", query), ("type", "hashtags"), ("limit", "20")]),
            )
            .await?;
            found.hashtags
        };
        Ok(tags
            .into_iter()
            .filter(|tag| !tag.name.is_empty() && valid_query(&tag.name))
            .take(20)
            .map(|tag| {
                let activity = tag
                    .history
                    .filter(|days| !days.is_empty())
                    .and_then(|days| {
                        let total = days.iter().try_fold(0u64, |sum, day| {
                            sum.checked_add(day.uses.parse::<u64>().ok()?)
                        })?;
                        Some(HashtagActivity::Mastodon {
                            uses: total,
                            days: days.len(),
                        })
                    })
                    .unwrap_or(HashtagActivity::Unavailable);
                HashtagSuggestion {
                    name: tag.name,
                    activity,
                }
            })
            .collect())
    }
}
