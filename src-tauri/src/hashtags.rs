use crate::{error::AppError, AppState};
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HashtagSuggestion {
    pub name: String,
    pub activity: HashtagActivity,
}
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "kind", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum HashtagActivity {
    Mastodon { uses: u64, days: usize },
    Bluesky { matches: Option<u64> },
    Unavailable,
}
pub fn valid_query(query: &str) -> bool {
    static PATTERN: std::sync::LazyLock<Result<regex::Regex, regex::Error>> =
        std::sync::LazyLock::new(|| regex::Regex::new(r"^[\p{L}\p{M}\p{N}_]*$"));
    query.chars().count() <= 64
        && PATTERN
            .as_ref()
            .is_ok_and(|pattern| pattern.is_match(query))
}
#[tauri::command]
pub async fn lookup_hashtags(
    account_id: String,
    query: String,
    state: State<'_, AppState>,
) -> Result<Vec<HashtagSuggestion>, AppError> {
    if !valid_query(&query) {
        return Err(AppError::Validation(
            "hashtag must contain at most 64 letters, numbers, or underscores".into(),
        ));
    }
    let provider = state
        .providers
        .read()
        .map_err(|_| AppError::StateUnavailable)?
        .get(&account_id)
        .cloned()
        .ok_or_else(|| AppError::Provider("Reconnect this account to look up hashtags".into()))?;
    provider.hashtags(&query).await
}
pub async fn response<T: for<'de> Deserialize<'de>>(
    request: reqwest::RequestBuilder,
) -> Result<T, AppError> {
    let response = request
        .timeout(std::time::Duration::from_secs(12))
        .send()
        .await
        .map_err(|_| {
            AppError::Provider("Hashtag lookup could not reach this provider. Try again.".into())
        })?;
    let status = response.status();
    if !status.is_success() {
        return Err(AppError::Provider(format!(
            "Hashtag lookup returned {status}. Try again or check account permissions."
        )));
    }
    response
        .json()
        .await
        .map_err(|_| AppError::Provider("The provider returned invalid hashtag data".into()))
}
