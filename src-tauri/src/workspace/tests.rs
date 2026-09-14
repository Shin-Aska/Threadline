use super::*;
use crate::accounts::mock_accounts;
use std::{
    collections::HashMap,
    sync::{Arc, RwLock},
};

#[derive(Default)]
struct MemoryCredentials(RwLock<HashMap<String, String>>);

impl CredentialStore for MemoryCredentials {
    fn set(&self, id: &str, secret: &str) -> Result<(), AppError> {
        self.0
            .write()
            .map_err(|_| AppError::StateUnavailable)?
            .insert(id.into(), secret.into());
        Ok(())
    }
    fn get(&self, id: &str) -> Result<String, AppError> {
        self.0
            .read()
            .map_err(|_| AppError::StateUnavailable)?
            .get(id)
            .cloned()
            .ok_or_else(|| AppError::Credential("missing".into()))
    }
    fn delete(&self, id: &str) -> Result<(), AppError> {
        self.0
            .write()
            .map_err(|_| AppError::StateUnavailable)?
            .remove(id);
        Ok(())
    }
}

fn real_account(id: &str) -> Account {
    let mut account = mock_accounts().remove(0);
    account.id = id.into();
    account
}

fn secret() -> &'static str {
    r#"{"provider":"BLUESKY","service_url":"https://example.invalid","identifier":"test","app_password":"test-only"}"#
}

fn state(database: Database, providers: ProviderMap) -> AppState {
    AppState {
        database,
        providers: RwLock::new(providers),
        credentials: Arc::new(MemoryCredentials::default()),
    }
}

#[test]
fn managed_account_restores_without_samples_when_environment_is_empty() {
    // Given a stored account and its available keychain credential.
    let database = Database::in_memory().expect("database");
    let account = real_account("bsky-managed");
    database
        .seed(std::slice::from_ref(&account))
        .expect("account");
    let credentials = MemoryCredentials::default();
    credentials.set(&account.id, secret()).expect("credential");
    // When startup restores the workspace.
    let providers =
        initialize(&database, &credentials, (Vec::new(), ProviderMap::new())).expect("initialize");
    // Then only the connected managed account is visible.
    let workspace = snapshot(&state(database, providers)).expect("snapshot");
    assert_eq!(workspace.accounts.len(), 1);
    assert_eq!(workspace.connected_account_ids, [account.id]);
    assert_eq!(workspace.mode, WorkspaceMode::Live);
}

#[test]
fn stored_account_stays_disconnected_when_credential_is_unavailable() {
    // Given a stored real account with no available credential.
    let database = Database::in_memory().expect("database");
    database
        .seed(&[real_account("bsky-managed")])
        .expect("account");
    // When startup restores the workspace.
    let providers = initialize(
        &database,
        &MemoryCredentials::default(),
        (Vec::new(), ProviderMap::new()),
    )
    .expect("initialize");
    // Then it retains the account without making a simulated connection.
    let workspace = snapshot(&state(database, providers)).expect("snapshot");
    assert_eq!(workspace.mode, WorkspaceMode::Disconnected);
    assert_eq!(workspace.accounts.len(), 1);
    assert!(workspace.connected_account_ids.is_empty());
}

#[test]
fn samples_are_removed_when_disconnected_real_account_exists() {
    // Given legacy samples alongside a real account whose keychain is unavailable.
    let database = Database::in_memory().expect("database");
    database.seed(&mock_accounts()).expect("samples");
    database
        .seed(&[real_account("bsky-managed")])
        .expect("account");
    // When startup initializes the workspace.
    initialize(
        &database,
        &MemoryCredentials::default(),
        (Vec::new(), ProviderMap::new()),
    )
    .expect("initialize");
    // Then the real account remains alone.
    let accounts = database.accounts().expect("accounts");
    assert_eq!(accounts.len(), 1);
    assert_eq!(accounts[0].id, "bsky-managed");
}

#[test]
fn fresh_workspace_has_no_accounts_without_credentials() {
    // Given a new empty database and no configured credentials.
    let database = Database::in_memory().expect("database");
    // When startup initializes the workspace.
    let providers = initialize(
        &database,
        &MemoryCredentials::default(),
        (Vec::new(), ProviderMap::new()),
    )
    .expect("initialize");
    let workspace = snapshot(&state(database, providers)).expect("snapshot");
    assert_eq!(workspace.mode, WorkspaceMode::Disconnected);
    assert!(workspace.accounts.is_empty());
    assert!(workspace.connected_account_ids.is_empty());
}

#[test]
fn empty_workspace_is_disconnected_when_last_account_was_removed() {
    // Given a workspace after removal of its last sample.
    let database = Database::in_memory().expect("database");
    database.seed(&mock_accounts()).expect("samples");
    for account in mock_accounts() {
        database.delete_account(&account.id).expect("delete");
    }
    // When the frontend requests a snapshot.
    let workspace = snapshot(&state(database, ProviderMap::new())).expect("snapshot");
    // Then empty state is disconnected, not an active demo.
    assert_eq!(workspace.mode, WorkspaceMode::Disconnected);
    assert!(workspace.accounts.is_empty());
}

#[test]
fn environment_accounts_replace_samples_and_preserve_managed_accounts() {
    // Given samples, a managed account, and two environment connections.
    let database = Database::in_memory().expect("database");
    database.seed(&mock_accounts()).expect("samples");
    database.seed(&[real_account("managed")]).expect("managed");
    let accounts = vec![real_account("z-env"), real_account("a-env")];
    let mut providers = ProviderMap::new();
    for account in &accounts {
        providers.insert(
            account.id.clone(),
            provider_from_credential(account, secret()).expect("provider"),
        );
    }
    // When startup imports environment connections.
    let providers = initialize(
        &database,
        &MemoryCredentials::default(),
        (accounts, providers),
    )
    .expect("initialize");
    // Then all real accounts remain with stable connected IDs.
    let workspace = snapshot(&state(database, providers)).expect("snapshot");
    assert_eq!(workspace.accounts.len(), 3);
    assert!(workspace
        .accounts
        .iter()
        .all(|account| !is_mock_account(&account.id)));
    assert_eq!(workspace.connected_account_ids, ["a-env", "z-env"]);
    assert_eq!(workspace.mode, WorkspaceMode::Live);
}

#[test]
fn disconnected_environment_account_does_not_seed_samples() {
    // Given account metadata from the environment but no provider.
    let database = Database::in_memory().expect("database");
    // When initialization imports that metadata.
    let providers = initialize(
        &database,
        &MemoryCredentials::default(),
        (vec![real_account("bsky-env")], ProviderMap::new()),
    )
    .expect("initialize");
    // Then it is stored as a disconnected account.
    let workspace = snapshot(&state(database, providers)).expect("snapshot");
    assert_eq!(workspace.mode, WorkspaceMode::Disconnected);
    assert_eq!(workspace.accounts.len(), 1);
    assert_eq!(workspace.accounts[0].id, "bsky-env");
}

#[test]
fn snapshot_serializes_contract_when_disconnected() {
    // Given an empty workspace.
    let app = state(Database::in_memory().expect("database"), ProviderMap::new());
    // When its snapshot crosses the IPC boundary.
    let value = serde_json::to_value(snapshot(&app).expect("snapshot")).expect("serialize");
    // Then the frontend receives camelCase keys and the explicit mode.
    assert_eq!(
        value,
        serde_json::json!({"accounts": [], "connectedAccountIds": [], "mode": "DISCONNECTED"})
    );
}

#[test]
fn legacy_sample_only_workspace_is_cleared_and_stays_empty_after_restart() {
    let database = Database::in_memory().expect("database");
    database.seed(&mock_accounts()).expect("samples");
    for _ in 0..2 {
        let providers = initialize(
            &database,
            &MemoryCredentials::default(),
            (Vec::new(), ProviderMap::new()),
        )
        .expect("initialize");
        assert!(providers.is_empty());
        assert!(database.accounts().expect("accounts").is_empty());
    }
}
