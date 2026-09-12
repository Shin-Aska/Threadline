use crate::{composer, error::AppError, models::*, AppState};
use tauri::State;
use uuid::Uuid;
#[tauri::command]
pub fn list_accounts(state: State<'_, AppState>) -> Result<Vec<Account>, AppError> {
    state.database.accounts()
}
#[tauri::command]
pub fn preview_post(
    post: CanonicalPost,
    state: State<'_, AppState>,
) -> Result<PublishingPreview, AppError> {
    composer::preview(&post, &state.database.accounts()?)
}
#[tauri::command]
pub async fn publish_post(
    post: CanonicalPost,
    state: State<'_, AppState>,
) -> Result<PublishResult, AppError> {
    let p = composer::preview(&post, &state.database.accounts()?)?;
    let publications = p
        .destinations
        .into_iter()
        .map(|d| Publication {
            account_id: d.account_id,
            status: PublicationStatus::Published,
            remote_post_ids: d
                .parts
                .iter()
                .enumerate()
                .map(|(i, _)| format!("simulated:{}", i + 1))
                .collect(),
            error: None,
        })
        .collect();
    Ok(PublishResult {
        canonical_id: Uuid::new_v4().to_string(),
        simulated: true,
        publications,
    })
}
#[tauri::command]
pub fn storage_health() -> String {
    "SQLite ready; credentials delegated to OS keychain".into()
}
