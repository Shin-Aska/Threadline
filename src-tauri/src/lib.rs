pub mod accounts;
pub mod commands;
pub mod composer;
pub mod config;
pub mod credentials;
pub mod database;
pub mod error;
pub mod models;
pub mod providers;
use credentials::{CredentialStore, OsKeychainCredentialStore};
use database::Database;
use std::sync::Arc;
use std::sync::RwLock;
pub struct AppState {
    pub database: Database,
    pub providers: RwLock<config::ProviderMap>,
    pub credentials: Arc<dyn CredentialStore>,
}
#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            use tauri::Manager;
            let dir = app.path().app_data_dir()?;
            std::fs::create_dir_all(&dir)?;
            let db = Database::open(&dir.join("threadline.sqlite"))
                .map_err(|e| Box::<dyn std::error::Error>::from(e.to_string()))?;
            let (live_accounts, mut providers) = config::live_accounts();
            let credentials: Arc<dyn CredentialStore> = Arc::new(OsKeychainCredentialStore);
            for account in db.accounts().unwrap_or_default() {
                if !providers.contains_key(&account.id) {
                    if let Ok(secret) = credentials.get(&account.id) {
                        if let Some(provider) = config::provider_from_credential(&account, &secret)
                        {
                            providers.insert(account.id.clone(), provider);
                        }
                    }
                }
            }
            if !providers.is_empty() {
                for id in ["bsky-alice", "mastodon-social", "mastodon-long"] {
                    db.delete_account(id)
                        .map_err(|e| Box::<dyn std::error::Error>::from(e.to_string()))?;
                }
            }
            let accounts = if live_accounts.is_empty() {
                accounts::mock_accounts()
            } else {
                live_accounts
            };
            if providers.is_empty() && db.accounts().map(|a| a.is_empty()).unwrap_or(true) {
                db.seed(&accounts::mock_accounts())
                    .map_err(|e| Box::<dyn std::error::Error>::from(e.to_string()))?;
            } else {
                for account in &accounts {
                    db.upsert_account(account)
                        .map_err(|e| Box::<dyn std::error::Error>::from(e.to_string()))?;
                }
            }
            app.manage(AppState {
                database: db,
                providers: RwLock::new(providers),
                credentials,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_accounts,
            commands::preview_post,
            commands::publish_post,
            commands::connect_bluesky,
            commands::connect_mastodon,
            commands::remove_account,
            commands::storage_health
        ])
        .run(tauri::generate_context!())
        .expect("error while running Threadline")
}
