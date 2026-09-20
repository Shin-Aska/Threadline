mod account_identity;
pub mod browsing;
pub mod publishing;
use crate::providers::{bluesky::BlueskyProvider, mastodon::MastodonProvider};
use crate::{composer, error::AppError, models::*, AppState};
use std::sync::Arc;
use tauri::State;
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
    publish_to_accounts(post, &state).await
}

pub async fn publish_to_accounts(
    post: CanonicalPost,
    state: &AppState,
) -> Result<PublishResult, AppError> {
    let draft = state.database.save_draft(SaveDraftInput {
        id: None,
        expected_revision: None,
        post,
    })?;
    let result = crate::publishing::publish_draft(state, &draft.id).await?;
    let canonical_id = result.id;
    let publications = result
        .destinations
        .into_iter()
        .map(|destination| Publication {
            account_id: destination.account_id,
            status: match destination.status {
                PublicationOutcome::Published => PublicationStatus::Published,
                PublicationOutcome::Failed => PublicationStatus::Failed,
                PublicationOutcome::Uncertain
                | PublicationOutcome::Pending
                | PublicationOutcome::InFlight => PublicationStatus::Uncertain,
                PublicationOutcome::Blocked => PublicationStatus::Blocked,
            },
            remote_post_ids: destination.remote_post_ids,
            error: destination.error,
        })
        .collect();
    Ok(PublishResult {
        canonical_id,
        publications,
    })
}

pub(crate) fn default_capabilities(max_text_length: usize, mastodon: bool) -> PlatformCapabilities {
    PlatformCapabilities {
        max_text_length,
        counting_policy: CountingPolicy::Grapheme,
        reserved_url_length: mastodon.then_some(23),
        max_media_attachments: 4,
        supported_media_types: vec![
            "image/jpeg".into(),
            "image/png".into(),
            "image/webp".into(),
            "video/mp4".into(),
        ],
        max_video_bytes: (!mastodon).then_some(300_000_000),
        max_video_duration_ms: None,
        supports_polls: mastodon,
        supports_content_warnings: mastodon,
    }
}

pub(crate) fn remove_mock_accounts(state: &AppState) -> Result<(), AppError> {
    if state
        .providers
        .read()
        .map_err(|_| AppError::StateUnavailable)?
        .is_empty()
    {
        for id in ["bsky-alice", "mastodon-social", "mastodon-long"] {
            state.database.delete_account(id)?;
        }
    }
    Ok(())
}

#[tauri::command]
pub async fn connect_bluesky(
    service_url: String,
    identifier: String,
    app_password: String,
    state: State<'_, AppState>,
) -> Result<Account, AppError> {
    connect_bluesky_account(service_url, identifier, app_password, &state).await
}
pub(crate) async fn connect_bluesky_account(
    service_url: String,
    identifier: String,
    app_password: String,
    state: &AppState,
) -> Result<Account, AppError> {
    if identifier.trim().is_empty() || app_password.trim().is_empty() {
        return Err(AppError::Validation(
            "Bluesky handle and app password are required".into(),
        ));
    }
    let provider = BlueskyProvider {
        app_password_session: Default::default(),
        capabilities: default_capabilities(300, false),
        client: reqwest::Client::new(),
        service_url,
        identifier,
        app_password,
        oauth: None,
    };
    let (did, handle) = provider.account().await?;
    let account = Account {
        id: format!("bsky-{did}"),
        provider: ProviderKind::Bluesky,
        handle: handle.clone(),
        display_name: handle,
        instance_url: Some(provider.service_url.clone()),
        did: Some(did),
        capabilities: provider.capabilities.clone(),
    };
    let credential = serde_json::to_string(&crate::config::StoredCredential::Bluesky {
        service_url: provider.service_url.clone(),
        identifier: provider.identifier.clone(),
        app_password: provider.app_password.clone(),
    })
    .map_err(|error| AppError::Credential(error.to_string()))?;
    state.credentials.set(&account.id, &credential)?;
    remove_mock_accounts(state)?;
    state.database.upsert_account(&account)?;
    state
        .providers
        .write()
        .map_err(|_| AppError::StateUnavailable)?
        .insert(account.id.clone(), Arc::new(provider));
    Ok(account)
}

#[tauri::command]
pub async fn connect_mastodon(
    base_url: String,
    access_token: String,
    state: State<'_, AppState>,
) -> Result<Account, AppError> {
    connect_mastodon_account(base_url, access_token, &state).await
}

pub(crate) async fn connect_mastodon_account(
    base_url: String,
    access_token: String,
    state: &AppState,
) -> Result<Account, AppError> {
    if base_url.trim().is_empty() || access_token.trim().is_empty() {
        return Err(AppError::Validation(
            "Mastodon server and access token are required".into(),
        ));
    }
    let mut provider = MastodonProvider {
        capabilities: default_capabilities(500, true),
        client: reqwest::Client::new(),
        base_url: base_url.trim_end_matches('/').to_owned(),
        access_token,
    };
    let (remote_id, handle, display_name) = provider.account().await?;
    provider.capabilities = provider.discovered_capabilities().await;
    let account = Account {
        id: account_identity::mastodon_account_id(
            &provider.base_url,
            &remote_id,
            &state.database.accounts()?,
        )?,
        provider: ProviderKind::Mastodon,
        handle,
        display_name,
        instance_url: Some(provider.base_url.clone()),
        did: None,
        capabilities: provider.capabilities.clone(),
    };
    let credential = serde_json::to_string(&crate::config::StoredCredential::Mastodon {
        base_url: provider.base_url.clone(),
        access_token: provider.access_token.clone(),
    })
    .map_err(|error| AppError::Credential(error.to_string()))?;
    state.credentials.set(&account.id, &credential)?;
    remove_mock_accounts(state)?;
    state.database.upsert_account(&account)?;
    state
        .providers
        .write()
        .map_err(|_| AppError::StateUnavailable)?
        .insert(account.id.clone(), Arc::new(provider));
    Ok(account)
}

#[tauri::command]
pub fn remove_account(account_id: String, state: State<'_, AppState>) -> Result<(), AppError> {
    let _ = state.credentials.delete(&account_id);
    state.database.delete_account(&account_id)?;
    state
        .providers
        .write()
        .map_err(|_| AppError::StateUnavailable)?
        .remove(&account_id);
    Ok(())
}

#[tauri::command]
pub fn storage_health() -> String {
    "SQLite ready; credentials delegated to OS keychain".into()
}

#[cfg(test)]
mod tests;

#[cfg(test)]
mod media_tests;
