use crate::providers::SocialProvider;
use crate::providers::{bluesky::BlueskyProvider, mastodon::MastodonProvider};
use crate::{composer, error::AppError, models::*, AppState};
use std::sync::Arc;
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
    let providers = state
        .providers
        .read()
        .map_err(|_| AppError::StateUnavailable)?
        .clone();
    let simulated = providers.is_empty();
    let mut publications = Vec::with_capacity(p.destinations.len());
    for destination in p.destinations {
        let Some(provider) = providers.get(&destination.account_id) else {
            publications.push(if simulated {
                simulated_publication(destination.account_id, &destination.parts)
            } else {
                Publication {
                    account_id: destination.account_id,
                    status: PublicationStatus::Failed,
                    remote_post_ids: Vec::new(),
                    error: Some("Account is not connected; reconnect it in Accounts".into()),
                }
            });
            continue;
        };
        publications.push(
            publish_destination(
                destination.account_id,
                &destination.parts,
                provider.as_ref(),
            )
            .await,
        );
    }
    Ok(PublishResult {
        canonical_id: Uuid::new_v4().to_string(),
        simulated,
        publications,
    })
}

fn default_capabilities(max_text_length: usize, mastodon: bool) -> PlatformCapabilities {
    PlatformCapabilities {
        max_text_length,
        counting_policy: CountingPolicy::Grapheme,
        reserved_url_length: mastodon.then_some(23),
        max_media_attachments: 4,
        supported_media_types: vec!["image/jpeg".into(), "image/png".into(), "video/mp4".into()],
        supports_polls: mastodon,
        supports_content_warnings: mastodon,
    }
}

fn remove_mock_accounts(state: &AppState) -> Result<(), AppError> {
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
    if identifier.trim().is_empty() || app_password.trim().is_empty() {
        return Err(AppError::Validation(
            "Bluesky handle and app password are required".into(),
        ));
    }
    let provider = BlueskyProvider {
        capabilities: default_capabilities(300, false),
        client: reqwest::Client::new(),
        service_url,
        identifier,
        app_password,
    };
    let (did, handle) = provider.account().await?;
    let account = Account {
        id: format!("bsky-{did}"),
        provider: ProviderKind::Bluesky,
        handle: handle.clone(),
        display_name: handle,
        instance_url: None,
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
    remove_mock_accounts(&state)?;
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
    if base_url.trim().is_empty() || access_token.trim().is_empty() {
        return Err(AppError::Validation(
            "Mastodon server and access token are required".into(),
        ));
    }
    let provider = MastodonProvider {
        capabilities: default_capabilities(500, true),
        client: reqwest::Client::new(),
        base_url: base_url.trim_end_matches('/').to_owned(),
        access_token,
    };
    let (remote_id, handle, display_name) = provider.account().await?;
    let account = Account {
        id: format!("mastodon-{remote_id}"),
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
    remove_mock_accounts(&state)?;
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

fn simulated_publication(account_id: String, parts: &[String]) -> Publication {
    Publication {
        account_id,
        status: PublicationStatus::Published,
        remote_post_ids: parts
            .iter()
            .enumerate()
            .map(|(index, _)| format!("simulated:{}", index + 1))
            .collect(),
        error: None,
    }
}

async fn publish_destination(
    account_id: String,
    parts: &[String],
    provider: &dyn SocialProvider,
) -> Publication {
    let mut published = Vec::new();
    let mut parent = None;
    for text in parts {
        let post = PreparedPost { text: text.clone() };
        let result = match parent.as_ref() {
            Some(parent) => provider.reply(parent, post).await,
            None => provider.publish(post).await,
        };
        match result {
            Ok(remote) => {
                published.push(remote.remote_id.clone());
                parent = Some(remote);
            }
            Err(error) => {
                return Publication {
                    account_id,
                    status: PublicationStatus::Failed,
                    remote_post_ids: published,
                    error: Some(error.to_string()),
                };
            }
        }
    }
    Publication {
        account_id,
        status: PublicationStatus::Published,
        remote_post_ids: published,
        error: None,
    }
}
#[tauri::command]
pub fn storage_health() -> String {
    "SQLite ready; credentials delegated to OS keychain".into()
}
