//! Restores connected providers and exposes the persisted workspace snapshot.
use crate::{
    config::{provider_from_credential_with_persistence, ProviderMap},
    credentials::CredentialStore,
    database::Database,
    error::AppError,
    models::{Account, WorkspaceMode, WorkspaceState},
    AppState,
};
use std::sync::Arc;

/// Identifies the bundled sample account IDs removed during initialization.
pub fn is_mock_account(id: &str) -> bool {
    matches!(id, "bsky-alice" | "mastodon-social" | "mastodon-long")
}

/// Restores stored credentials, removes sample accounts, and registers environment accounts.
///
/// Accounts without usable credentials remain persisted but disconnected.
pub async fn initialize(
    database: &Database,
    credentials: Arc<dyn CredentialStore>,
    environment: (Vec<Account>, ProviderMap),
) -> Result<ProviderMap, AppError> {
    let (live_accounts, mut providers) = environment;
    let stored_accounts = database.accounts()?;
    for account in &stored_accounts {
        if !is_mock_account(&account.id) && !providers.contains_key(&account.id) {
            if let Ok(secret) = credentials.get(&account.id) {
                if let Some(provider) = provider_from_credential_with_persistence(
                    account,
                    &secret,
                    Some(Arc::clone(&credentials)),
                )
                .await
                {
                    providers.insert(account.id.clone(), provider);
                }
            }
        }
    }
    for id in ["bsky-alice", "mastodon-social", "mastodon-long"] {
        database.delete_account(id)?;
        providers.remove(id);
    }
    for account in &live_accounts {
        if !is_mock_account(&account.id) {
            database.upsert_account(account)?;
        }
    }
    Ok(providers)
}

/// Returns stored accounts and the IDs of currently connected providers.
pub fn snapshot(state: &AppState) -> Result<WorkspaceState, AppError> {
    let accounts = state.database.accounts()?;
    let providers = state
        .providers
        .read()
        .map_err(|_| AppError::StateUnavailable)?;
    let mut connected_account_ids: Vec<String> = providers.keys().cloned().collect();
    connected_account_ids.sort_unstable();
    let mode = if !providers.is_empty() {
        WorkspaceMode::Live
    } else {
        WorkspaceMode::Disconnected
    };
    Ok(WorkspaceState {
        accounts,
        connected_account_ids,
        mode,
    })
}

#[tauri::command]
/// Exposes the current workspace snapshot to the frontend.
pub fn get_workspace(state: tauri::State<'_, AppState>) -> Result<WorkspaceState, AppError> {
    snapshot(&state)
}

#[cfg(test)]
/// Covers restoration, disconnected records, sample cleanup, and snapshot shape.
mod tests;
