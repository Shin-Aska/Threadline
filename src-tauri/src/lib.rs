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
pub mod providers;
pub mod workspace;
use credentials::{CredentialStore, OsKeychainCredentialStore};
use database::Database;
use std::sync::Arc;
use std::sync::RwLock;
pub struct AppState {
    pub database: Database,
    pub providers: RwLock<config::ProviderMap>,
    pub credentials: Arc<dyn CredentialStore>,
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
            let credentials: Arc<dyn CredentialStore> = Arc::new(OsKeychainCredentialStore);
            let providers =
                workspace::initialize(&db, credentials.as_ref(), config::live_accounts())
                    .map_err(|e| Box::<dyn std::error::Error>::from(e.to_string()))?;
            app.manage(AppState {
                database: db,
                providers: RwLock::new(providers),
                credentials,
            });
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
            commands::storage_health
        ])
        .run(tauri::generate_context!())
        .expect("error while running Threadline")
}
