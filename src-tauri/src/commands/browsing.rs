use crate::{error::AppError, AppState};
use tauri::State;
fn provider_and_account<'a>(
    account_id: &str,
    state: &'a AppState,
) -> Result<
    (
        std::sync::Arc<dyn crate::providers::SocialProvider>,
        crate::models::Account,
    ),
    AppError,
> {
    let account = state
        .database
        .accounts()?
        .into_iter()
        .find(|a| a.id == account_id)
        .ok_or_else(|| AppError::Validation("Unknown account".into()))?;
    let provider = state
        .providers
        .read()
        .map_err(|_| AppError::StateUnavailable)?
        .get(account_id)
        .cloned()
        .ok_or_else(|| {
            AppError::Provider("Account is disconnected; reconnect it in Accounts & Sync".into())
        })?;
    Ok((provider, account))
}
#[tauri::command]
pub async fn get_timeline(
    account_id: String,
    cursor: Option<String>,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, AppError> {
    let (p, a) = provider_and_account(&account_id, &state)?;
    p.timeline(&a.id, &a.handle, cursor.as_deref()).await
}
#[tauri::command]
pub async fn get_discovery(
    account_id: String,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, AppError> {
    let (p, a) = provider_and_account(&account_id, &state)?;
    p.discovery(&a.id, &a.handle).await
}
#[tauri::command]
pub async fn get_following_sources(
    account_id: String,
    state: State<'_, AppState>,
) -> Result<serde_json::Value, AppError> {
    let (p, _) = provider_and_account(&account_id, &state)?;
    p.following_sources(&account_id).await
}
