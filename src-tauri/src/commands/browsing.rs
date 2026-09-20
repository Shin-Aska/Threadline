use crate::{error::AppError, AppState};
use tauri::State;
fn provider_and_account(
    account_id: &str,
    state: &AppState,
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

#[tauri::command]
pub async fn get_home_feed(
    account_id: String,
    cursor: Option<String>,
    state: State<'_, AppState>,
) -> Result<crate::providers::social::FeedPage, AppError> {
    let (provider, _) = provider_and_account(&account_id, &state)?;
    provider.home_feed(cursor.as_deref()).await
}

#[tauri::command]
pub async fn get_own_feed(
    account_id: String,
    kind: crate::providers::social::ProfileFeedKind,
    cursor: Option<String>,
    state: State<'_, AppState>,
) -> Result<crate::providers::social::FeedPage, AppError> {
    let (provider, _) = provider_and_account(&account_id, &state)?;
    provider.own_feed(kind, cursor.as_deref()).await
}

#[tauri::command]
pub async fn get_own_profile(
    account_id: String,
    state: State<'_, AppState>,
) -> Result<crate::providers::social::ProfileDetails, AppError> {
    let (provider, _) = provider_and_account(&account_id, &state)?;
    provider.own_profile().await
}

#[tauri::command]
pub async fn get_profile(
    account_id: String,
    profile_id: String,
    state: State<'_, AppState>,
) -> Result<crate::providers::social::ProfileDetails, AppError> {
    let (provider, _) = provider_and_account(&account_id, &state)?;
    provider.profile(&profile_id).await
}

#[tauri::command]
pub async fn get_profile_feed(
    account_id: String,
    profile_id: String,
    kind: crate::providers::social::ProfileFeedKind,
    cursor: Option<String>,
    state: State<'_, AppState>,
) -> Result<crate::providers::social::FeedPage, AppError> {
    let (provider, _) = provider_and_account(&account_id, &state)?;
    provider
        .profile_feed(&profile_id, kind, cursor.as_deref())
        .await
}

#[tauri::command]
pub async fn get_thread(
    account_id: String,
    post_id: String,
    state: State<'_, AppState>,
) -> Result<crate::providers::social::ThreadView, AppError> {
    let (provider, _) = provider_and_account(&account_id, &state)?;
    provider.thread(&post_id).await
}

#[tauri::command]
pub async fn get_tag_feed(
    account_id: String,
    tag: String,
    cursor: Option<String>,
    state: State<'_, AppState>,
) -> Result<crate::providers::social::FeedPage, AppError> {
    let (provider, _) = provider_and_account(&account_id, &state)?;
    provider
        .tag_feed(tag.trim_start_matches('#'), cursor.as_deref())
        .await
}

#[tauri::command]
pub async fn get_followed_sources(
    account_id: String,
    cursor: Option<String>,
    state: State<'_, AppState>,
) -> Result<crate::providers::social::SourcePage, AppError> {
    let (provider, _) = provider_and_account(&account_id, &state)?;
    provider.followed_sources_page(cursor.as_deref()).await
}

#[tauri::command]
pub async fn get_source_feed(
    account_id: String,
    source: crate::providers::social::FollowedSource,
    cursor: Option<String>,
    state: State<'_, AppState>,
) -> Result<crate::providers::social::FeedPage, AppError> {
    let (provider, account) = provider_and_account(&account_id, &state)?;
    if source.provider != account.provider {
        return Err(AppError::Validation(
            "Source belongs to another provider".into(),
        ));
    }
    provider.source_feed(&source, cursor.as_deref()).await
}

#[tauri::command]
pub async fn get_notifications(
    account_id: String,
    cursor: Option<String>,
    state: State<'_, AppState>,
) -> Result<crate::providers::social::NotificationPage, AppError> {
    let (provider, account) = provider_and_account(&account_id, &state)?;
    let mut page = provider.notifications(cursor.as_deref()).await?;
    if matches!(account.provider, crate::models::ProviderKind::Mastodon) {
        let read_ids = state.database.notification_read_ids(&account_id)?;
        for notification in &mut page.notifications {
            notification.unread = !read_ids.contains(&notification.id);
        }
    }
    Ok(page)
}

#[tauri::command]
pub async fn mark_notifications_read(
    account_id: String,
    notification_ids: Vec<String>,
    state: State<'_, AppState>,
) -> Result<(), AppError> {
    let (provider, account) = provider_and_account(&account_id, &state)?;
    match account.provider {
        crate::models::ProviderKind::Mastodon => state
            .database
            .mark_notifications_read_local(&account_id, &notification_ids),
        crate::models::ProviderKind::Bluesky => {
            provider.mark_notifications_read(&notification_ids).await
        }
    }
}

#[tauri::command]
pub async fn perform_social_action(
    account_id: String,
    action: crate::providers::social::SocialAction,
    state: State<'_, AppState>,
) -> Result<crate::providers::social::SocialActionResult, AppError> {
    let (provider, _) = provider_and_account(&account_id, &state)?;
    provider.social_action(action).await
}
