use super::BlueskyProvider;
use crate::{
    error::AppError,
    hashtags::{HashtagActivity, HashtagSuggestion},
};
use serde::Deserialize;
use std::time::{Duration, Instant};

pub struct SearchSession {
    created: Instant,
    token: String,
}
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
        let token = {
            let mut cached = self.search_session.lock().await;
            if let Some(session) = cached
                .as_ref()
                .filter(|session| session.created.elapsed() < Duration::from_secs(1200))
            {
                session.token.clone()
            } else {
                let session = tokio::time::timeout(Duration::from_secs(12), self.session())
                    .await
                    .map_err(|_| {
                        AppError::Provider("Bluesky hashtag sign-in timed out".into())
                    })??;
                let token = session.access_jwt;
                *cached = Some(SearchSession {
                    created: Instant::now(),
                    token: token.clone(),
                });
                token
            }
        };
        let search = self
            .client
            .get(format!(
                "{}/xrpc/app.bsky.feed.searchPosts",
                self.service_url.trim_end_matches('/')
            ))
            .bearer_auth(token)
            .query(&[
                ("q", format!("#{query}")),
                ("tag", query.into()),
                ("limit", "1".into()),
            ])
            .timeout(Duration::from_secs(12))
            .send()
            .await
            .map_err(|_| {
                AppError::Provider("Bluesky hashtag lookup could not connect. Try again.".into())
            })?;
        let status = search.status();
        if status == reqwest::StatusCode::UNAUTHORIZED {
            *self.search_session.lock().await = None;
        }
        if !status.is_success() {
            return Err(AppError::Provider(format!(
                "Bluesky hashtag lookup returned {status}. Try again or reconnect the account."
            )));
        }
        let result: Search = search
            .json()
            .await
            .map_err(|_| AppError::Provider("Invalid Bluesky hashtag response".into()))?;
        Ok(vec![HashtagSuggestion {
            name: query.into(),
            activity: HashtagActivity::Bluesky {
                matches: result.hits_total,
            },
        }])
    }
}
