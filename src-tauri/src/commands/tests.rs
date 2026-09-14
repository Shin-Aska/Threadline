use super::*;
use crate::{
    accounts::mock_accounts, config::ProviderMap, credentials::CredentialStore, database::Database,
};
use std::sync::RwLock;

struct UnusedCredentials;
impl CredentialStore for UnusedCredentials {
    fn set(&self, _: &str, _: &str) -> Result<(), AppError> {
        Err(AppError::Credential("test store unavailable".into()))
    }
    fn get(&self, _: &str) -> Result<String, AppError> {
        Err(AppError::Credential("test store unavailable".into()))
    }
    fn delete(&self, _: &str) -> Result<(), AppError> {
        Err(AppError::Credential("test store unavailable".into()))
    }
}
fn test_state(accounts: &[Account]) -> AppState {
    let database = Database::in_memory().expect("database");
    database.seed(accounts).expect("accounts");
    AppState {
        database,
        providers: RwLock::new(ProviderMap::new()),
        credentials: Arc::new(UnusedCredentials),
    }
}
fn post(accounts: &[Account]) -> CanonicalPost {
    CanonicalPost {
        text: "A sufficiently long post. ".repeat(40),
        media: vec![],
        policy: PublishingPolicy::Adaptive,
        destination_account_ids: accounts.iter().map(|account| account.id.clone()).collect(),
    }
}
#[tokio::test]
async fn missing_real_credentials_fail_publication() {
    let mut account = mock_accounts().remove(0);
    account.id = "bsky-real".into();
    let accounts = vec![account];
    let state = test_state(&accounts);
    let result = publish_to_accounts(post(&accounts), &state)
        .await
        .expect("result");
    assert!(matches!(
        result.publications[0].status,
        PublicationStatus::Failed
    ));
    assert!(result.publications[0].remote_post_ids.is_empty());
}
#[tokio::test]
async fn unconnected_legacy_accounts_cannot_report_published() {
    let accounts = mock_accounts();
    let state = test_state(&accounts);
    let result = publish_to_accounts(post(&accounts), &state)
        .await
        .expect("result");
    assert_eq!(result.publications.len(), 3);
    assert!(result.publications.iter().all(|publication| matches!(
        publication.status,
        PublicationStatus::Failed
    ) && publication
        .remote_post_ids
        .is_empty()));
}

#[derive(Default)]
struct MemoryCredentials(std::sync::Mutex<std::collections::HashMap<String, String>>);
impl CredentialStore for MemoryCredentials {
    fn set(&self, id: &str, secret: &str) -> Result<(), AppError> {
        self.0
            .lock()
            .map_err(|_| AppError::StateUnavailable)?
            .insert(id.into(), secret.into());
        Ok(())
    }
    fn get(&self, id: &str) -> Result<String, AppError> {
        self.0
            .lock()
            .map_err(|_| AppError::StateUnavailable)?
            .get(id)
            .cloned()
            .ok_or_else(|| AppError::Credential("missing test credential".into()))
    }
    fn delete(&self, id: &str) -> Result<(), AppError> {
        self.0
            .lock()
            .map_err(|_| AppError::StateUnavailable)?
            .remove(id);
        Ok(())
    }
}

#[tokio::test]
async fn custom_bluesky_service_survives_storage_and_disconnected_reconnect() {
    use std::io::{BufRead, BufReader, Read, Write};
    // Given a custom service and an isolated credential store.
    let listener = std::net::TcpListener::bind("127.0.0.1:0").expect("bind");
    let service_url = format!("http://{}", listener.local_addr().expect("address"));
    let server = std::thread::spawn(move || {
        let mut requests = Vec::new();
        for _ in 0..2 {
            let (socket, _) = listener.accept().expect("accept");
            socket
                .set_read_timeout(Some(std::time::Duration::from_secs(2)))
                .expect("timeout");
            let mut reader = BufReader::new(socket);
            let mut first_line = String::new();
            reader.read_line(&mut first_line).expect("request line");
            assert_eq!(
                first_line,
                "POST /xrpc/com.atproto.server.createSession HTTP/1.1\r\n"
            );
            let mut content_length = None;
            loop {
                let mut line = String::new();
                reader.read_line(&mut line).expect("header");
                if line == "\r\n" {
                    break;
                }
                if let Some((name, value)) = line.split_once(':') {
                    if name.eq_ignore_ascii_case("content-length") {
                        content_length = Some(value.trim().parse::<usize>().expect("length"));
                    }
                }
            }
            let mut body = vec![0; content_length.expect("content length")];
            reader.read_exact(&mut body).expect("body");
            requests.push(serde_json::from_slice::<serde_json::Value>(&body).expect("JSON"));
            let body =
                r#"{"accessJwt":"test-token","did":"did:plc:custom","handle":"custom.test"}"#;
            write!(reader.get_mut(), "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).expect("response");
        }
        requests
    });
    let mut state = test_state(&mock_accounts());
    state.credentials = Arc::new(MemoryCredentials::default());
    // When the account is connected, loses its credential, and reconnects from stored metadata.
    let account = connect_bluesky_account(
        service_url.clone(),
        "custom.test".into(),
        "first-test-password".into(),
        &state,
    )
    .await
    .expect("connect");
    state
        .credentials
        .delete(&account.id)
        .expect("remove credential");
    state.providers.write().expect("providers").clear();
    let disconnected = crate::workspace::snapshot(&state).expect("snapshot");
    assert_eq!(disconnected.mode, WorkspaceMode::Disconnected);
    assert_eq!(disconnected.accounts.len(), 1);
    let stored = &disconnected.accounts[0];
    assert_eq!(stored.instance_url.as_deref(), Some(service_url.as_str()));
    connect_bluesky_account(
        stored.instance_url.clone().expect("stored service"),
        stored.handle.clone(),
        "second-test-password".into(),
        &state,
    )
    .await
    .expect("reconnect");
    // Then both credential submissions reach that service and the saved account remains connected.
    let requests = server.join().expect("server");
    assert_eq!(
        requests,
        [
            serde_json::json!({"identifier":"custom.test","password":"first-test-password"}),
            serde_json::json!({"identifier":"custom.test","password":"second-test-password"})
        ]
    );
    let workspace = crate::workspace::snapshot(&state).expect("snapshot");
    assert_eq!(workspace.mode, WorkspaceMode::Live);
    assert_eq!(
        workspace.accounts[0].instance_url.as_deref(),
        Some(service_url.as_str())
    );
}
