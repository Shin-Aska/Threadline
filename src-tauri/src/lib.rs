#[cfg(test)]
pub mod accounts;
pub mod commands;
pub mod composer;
pub mod config;
pub mod credentials;
pub mod database;
pub mod error;
pub mod hashtags;
pub mod media;
pub mod models;
pub mod oauth;
pub mod providers;
pub mod publishing;
pub mod scheduling;
pub mod workspace;
use credentials::{CredentialStore, OsKeychainCredentialStore};
use database::Database;
use std::sync::Arc;
use std::sync::RwLock;
pub struct AppState {
    pub database: Database,
    pub providers: RwLock<config::ProviderMap>,
    pub credentials: Arc<dyn CredentialStore>,
    pub oauth: oauth::coordinator::OAuthCoordinator,
}
#[cfg(target_os = "linux")]
fn configure_linux_webkit() {
    // WebKitGTK can create a healthy WebView that never paints when the Linux
    // display exposes only a software GL renderer. Prefer the reliable software
    // paint path so Threadline remains usable on virtualized and low-end desktops.
    std::env::set_var("WEBKIT_DISABLE_COMPOSITING_MODE", "1");
}
#[cfg(all(test, target_os = "linux"))]
mod startup_tests {
    #[test]
    fn linux_startup_disables_webkit_compositing() {
        super::configure_linux_webkit();
        assert_eq!(
            std::env::var("WEBKIT_DISABLE_COMPOSITING_MODE").as_deref(),
            Ok("1")
        );
    }
}
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(target_os = "linux")]
    configure_linux_webkit();

    tauri::Builder::default()
        .setup(|app| {
            use tauri::Manager;
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            let db = Database::open(&dir.join("threadline.sqlite"))
                .map_err(|e| Box::<dyn std::error::Error>::from(e.to_string()))?;
            db.recover_interrupted_publications()
                .map_err(|e| Box::<dyn std::error::Error>::from(e.to_string()))?;
            db.mark_startup_missed(database::publishing::now_epoch_ms())
                .map_err(|e| Box::<dyn std::error::Error>::from(e.to_string()))?;
            let credentials: Arc<dyn CredentialStore> = Arc::new(OsKeychainCredentialStore);
            let providers = tauri::async_runtime::block_on(workspace::initialize(
                &db,
                Arc::clone(&credentials),
                config::live_accounts(),
            ))
            .map_err(|e| Box::<dyn std::error::Error>::from(e.to_string()))?;
            app.manage(AppState {
                database: db,
                providers: RwLock::new(providers),
                credentials,
                oauth: oauth::coordinator::OAuthCoordinator::default(),
            });
            tauri::async_runtime::spawn(scheduling::run(app.handle().clone()));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_accounts,
            workspace::get_workspace,
            commands::preview_post,
            hashtags::lookup_hashtags,
            commands::publish_post,
            commands::connect_bluesky,
            commands::connect_mastodon,
            commands::remove_account,
            commands::storage_health,
            oauth::commands::connect_mastodon_oauth,
            oauth::commands::connect_bluesky_oauth,
            oauth::commands::cancel_oauth_login,
            commands::publishing::save_draft,
            commands::publishing::list_drafts,
            commands::publishing::get_draft,
            commands::publishing::delete_draft,
            commands::publishing::publish_draft,
            commands::publishing::list_publications,
            commands::publishing::delete_publication,
            commands::publishing::create_schedule,
            commands::publishing::list_schedules,
            commands::publishing::reschedule_publication,
            commands::publishing::cancel_schedule,
            commands::publishing::send_schedule_now,
            commands::browsing::get_timeline,
            commands::browsing::get_discovery,
            commands::browsing::get_following_sources,
            commands::browsing::get_home_feed,
            commands::browsing::get_own_feed,
            commands::browsing::get_own_profile,
            commands::browsing::get_profile,
            commands::browsing::get_profile_feed,
            commands::browsing::get_thread,
            commands::browsing::get_tag_feed,
            commands::browsing::get_followed_sources,
            commands::browsing::get_source_feed,
            commands::browsing::get_notifications,
            commands::browsing::mark_notifications_read,
            commands::browsing::perform_social_action
        ])
        .run(tauri::generate_context!())
        .expect("error while running Threadline")
}
