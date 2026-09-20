use super::{
    bluesky::{self, BlueskyLoginRequest},
    mastodon::{self, MastodonLoginRequest},
    OAuthError,
};
use crate::{
    commands::{connect_mastodon_account, default_capabilities, remove_mock_accounts},
    error::AppError,
    models::{Account, ProviderKind},
    providers::bluesky::BlueskyProvider,
    AppState,
};
use std::sync::Arc;
use tauri::State;

#[tauri::command]
pub async fn connect_mastodon_oauth(
    flow_id: String,
    instance_url: String,
    state: State<'_, AppState>,
) -> Result<Account, AppError> {
    let flow = state.oauth.start(&flow_id, false).map_err(app_error)?;
    let credential = mastodon::login(MastodonLoginRequest::new(instance_url), flow.cancel.clone())
        .await
        .map_err(app_error)?;
    let (base_url, access_token) = credential.into_provider_secret();
    connect_mastodon_account(base_url, access_token, &state).await
}

#[tauri::command]
pub async fn connect_bluesky_oauth(
    flow_id: String,
    identifier: String,
    client_metadata_url: Option<String>,
    state: State<'_, AppState>,
) -> Result<Account, AppError> {
    let hosted = client_metadata_url.is_some();
    let mut flow = state.oauth.start(&flow_id, hosted).map_err(app_error)?;
    let request = BlueskyLoginRequest::new(identifier, client_metadata_url);
    let credential = if hosted {
        let callback = flow.deep_link.take().ok_or_else(|| {
            AppError::Validation("Bluesky native callback is not configured".into())
        })?;
        bluesky::login_hosted(request, callback, flow.cancel.clone()).await
    } else {
        bluesky::login_localhost(request, flow.cancel.clone()).await
    }
    .map_err(app_error)?;
    let runtime = credential
        .restore_with_persistence(Some(Arc::clone(&state.credentials)))
        .await
        .map_err(app_error)?;
    let service_url = runtime.service_url();
    let capabilities = default_capabilities(300, false);
    let provider = BlueskyProvider {
        app_password_session: Default::default(),
        capabilities,
        client: reqwest::Client::new(),
        service_url: service_url.clone(),
        identifier: runtime.subject().into(),
        app_password: String::new(),
        oauth: Some(Arc::new(runtime)),
    };
    let (did, handle) = provider.account().await?;
    let account = Account {
        id: format!("bsky-{did}"),
        provider: ProviderKind::Bluesky,
        handle: handle.clone(),
        display_name: handle,
        instance_url: Some(service_url),
        did: Some(did),
        capabilities: provider.capabilities.clone(),
    };
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
pub fn cancel_oauth_login(flow_id: String, state: State<'_, AppState>) -> Result<(), AppError> {
    state.oauth.cancel(&flow_id).map_err(app_error)
}

pub fn deliver_deep_link(state: &AppState, url: String) -> Result<(), AppError> {
    state.oauth.deliver_deep_link(url).map_err(app_error)
}

fn app_error(error: OAuthError) -> AppError {
    match error {
        OAuthError::Configuration(message)
        | OAuthError::InvalidMetadata(message)
        | OAuthError::InvalidResponse(message) => AppError::Validation(message),
        OAuthError::Conflict(message) => AppError::Conflict(message),
        other => AppError::Provider(other.to_string()),
    }
}
