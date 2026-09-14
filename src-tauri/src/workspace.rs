use crate::{
    config::{provider_from_credential, ProviderMap},
    credentials::CredentialStore,
    database::Database,
    error::AppError,
    models::{Account, WorkspaceMode, WorkspaceState},
    AppState,
};

pub fn is_mock_account(id: &str) -> bool {
    matches!(id, "bsky-alice" | "mastodon-social" | "mastodon-long")
}

pub fn initialize(
    database: &Database,
    credentials: &dyn CredentialStore,
    environment: (Vec<Account>, ProviderMap),
) -> Result<ProviderMap, AppError> {
    let (live_accounts, mut providers) = environment;
    let stored_accounts = database.accounts()?;
    for account in &stored_accounts {
        if !is_mock_account(&account.id) && !providers.contains_key(&account.id) {
            if let Ok(secret) = credentials.get(&account.id) {
                if let Some(provider) = provider_from_credential(account, &secret) {
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
pub fn get_workspace(state: tauri::State<'_, AppState>) -> Result<WorkspaceState, AppError> {
    snapshot(&state)
}

#[cfg(test)]
mod tests;
