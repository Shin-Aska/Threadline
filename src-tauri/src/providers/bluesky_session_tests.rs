use super::{bluesky::BlueskyProvider, SocialProvider};
use std::{
    io::{BufRead, BufReader, Write},
    net::{TcpListener, TcpStream},
    time::{Duration, Instant},
};

fn bluesky(service_url: String) -> BlueskyProvider {
    BlueskyProvider {
        app_password_session: Default::default(),
        capabilities: crate::accounts::mock_accounts()[0].capabilities.clone(),
        client: reqwest::Client::builder()
            .no_proxy()
            .build()
            .expect("client"),
        service_url,
        identifier: "alice.test".into(),
        app_password: "secret".into(),
        oauth: None,
    }
}

fn read_request(socket: TcpStream) -> (BufReader<TcpStream>, String) {
    let mut reader = BufReader::new(socket);
    let mut headers = String::new();
    loop {
        let mut line = String::new();
        reader.read_line(&mut line).expect("request header");
        if line == "\r\n" {
            break;
        }
        headers.push_str(&line);
    }
    (reader, headers)
}

fn respond(reader: &mut BufReader<TcpStream>, status: u16, body: &str) {
    write!(
        reader.get_mut(),
        "HTTP/1.1 {status} Response\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
        body.len()
    )
    .expect("response");
}

fn stale_unauthorized_server() -> (String, std::thread::JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    let url = format!("http://{}", listener.local_addr().expect("address"));
    let task = std::thread::spawn(move || {
        let (first_socket, _) = listener.accept().expect("first old-token read");
        let (mut first, first_headers) = read_request(first_socket);
        let (second_socket, _) = listener.accept().expect("second old-token read");
        let (mut second, second_headers) = read_request(second_socket);
        respond(&mut second, 401, r#"{"error":"ExpiredToken"}"#);

        let (login_socket, _) = listener.accept().expect("renewed login");
        let (mut login, login_headers) = read_request(login_socket);
        respond(
            &mut login,
            200,
            r#"{"accessJwt":"new-jwt","did":"did:plc:alice","handle":"alice.test"}"#,
        );
        let (second_retry_socket, _) = listener.accept().expect("second read retry");
        let (mut second_retry, second_retry_headers) = read_request(second_retry_socket);
        respond(&mut second_retry, 200, r#"{"feed":[]}"#);

        respond(&mut first, 401, r#"{"error":"ExpiredToken"}"#);
        let (first_retry_socket, _) = listener.accept().expect("first read retry");
        let (mut first_retry, first_retry_headers) = read_request(first_retry_socket);
        respond(&mut first_retry, 200, r#"{"feed":[]}"#);

        vec![
            first_headers,
            second_headers,
            login_headers,
            second_retry_headers,
            first_retry_headers,
        ]
    });
    (url, task)
}

#[tokio::test]
async fn concurrent_bluesky_reads_share_one_app_password_login() {
    // Given two reads start before an app-password session exists.
    let session = r#"{"accessJwt":"jwt","did":"did:plc:alice","handle":"alice.test"}"#;
    let feed = r#"{"feed":[]}"#;
    let (url, server) = super::test_http::server(vec![(200, session), (200, feed), (200, feed)]);
    let provider = bluesky(url);

    // When both reads request authentication concurrently.
    let (first, second) = tokio::join!(provider.home_feed(None), provider.home_feed(None));

    // Then session creation is coalesced and both reads complete with that session.
    first.expect("first feed");
    second.expect("second feed");
    let requests = server.join().expect("server");
    assert_eq!(
        requests
            .iter()
            .filter(|request| request.headers.contains("com.atproto.server.createSession"))
            .count(),
        1
    );
    assert_eq!(requests.len(), 3);
}

#[tokio::test]
async fn concurrent_bluesky_reads_share_recent_login_failure() {
    // Given the app-password login endpoint rejects the credentials.
    let (url, server) = super::test_http::server(vec![(401, r#"{"error":"InvalidIdentifier"}"#)]);
    let provider = bluesky(url);

    // When two reads request authentication concurrently.
    let (first, second) = tokio::join!(provider.home_feed(None), provider.home_feed(None));

    // Then both reads fail without repeating the rejected login request.
    assert!(first.is_err());
    assert!(second.is_err());
    assert_eq!(server.join().expect("server").len(), 1);
}

#[tokio::test]
async fn expired_app_password_session_is_replaced_before_read() {
    // Given a provider session older than the conservative reuse window.
    let renewed = r#"{"accessJwt":"new-jwt","did":"did:plc:alice","handle":"alice.test"}"#;
    let feed = r#"{"feed":[]}"#;
    let (url, server) = super::test_http::server(vec![(200, renewed), (200, feed)]);
    let provider = bluesky(url);
    *provider.app_password_session.lock().await = super::bluesky::AppPasswordSessionState::Active {
        created: Instant::now() - Duration::from_secs(20 * 60 + 1),
        session: super::bluesky::SessionResponse {
            access_jwt: "old-jwt".into(),
            did: "did:plc:alice".into(),
            handle: "alice.test".into(),
        },
    };

    // When an authenticated read starts.
    provider.home_feed(None).await.expect("feed");

    // Then the read signs in once and uses only the renewed token.
    let requests = server.join().expect("server");
    assert_eq!(requests.len(), 2);
    assert!(requests[1]
        .headers
        .to_lowercase()
        .contains("authorization: bearer new-jwt"));
}

#[tokio::test]
async fn unauthorized_write_invalidates_session_without_retrying_write() {
    // Given an authenticated write returns unauthorized before a later read.
    let old = r#"{"accessJwt":"old-jwt","did":"did:plc:alice","handle":"alice.test"}"#;
    let renewed = r#"{"accessJwt":"new-jwt","did":"did:plc:alice","handle":"alice.test"}"#;
    let feed = r#"{"feed":[]}"#;
    let (url, server) = super::test_http::server(vec![
        (200, old),
        (401, r#"{"error":"ExpiredToken"}"#),
        (200, renewed),
        (200, feed),
    ]);
    let provider = bluesky(url);

    // When the write fails and a read follows.
    assert!(provider
        .mark_notifications_read(&["notification-1".into()])
        .await
        .is_err());
    provider.home_feed(None).await.expect("feed");

    // Then the write was attempted once and the read renewed the invalid session.
    let requests = server.join().expect("server");
    assert_eq!(
        requests
            .iter()
            .filter(|request| request.headers.contains("notification.updateSeen"))
            .count(),
        1
    );
    assert_eq!(
        requests
            .iter()
            .filter(|request| request.headers.contains("com.atproto.server.createSession"))
            .count(),
        2
    );
    assert!(requests[3]
        .headers
        .to_lowercase()
        .contains("authorization: bearer new-jwt"));
}

#[tokio::test]
async fn delayed_old_unauthorized_response_does_not_clear_renewed_session() {
    // Given two reads use an old token and one unauthorized response is delayed.
    let (url, server) = stale_unauthorized_server();
    let provider = bluesky(url);
    *provider.app_password_session.lock().await = super::bluesky::AppPasswordSessionState::Active {
        created: Instant::now(),
        session: super::bluesky::SessionResponse {
            access_jwt: "old-jwt".into(),
            did: "did:plc:alice".into(),
            handle: "alice.test".into(),
        },
    };

    // When both reads recover from unauthorized responses around one renewal.
    let (first, second) = tokio::join!(provider.home_feed(None), provider.home_feed(None));

    // Then both retry with the renewed token and only one login occurs.
    first.expect("first feed");
    second.expect("second feed");
    let requests = server.join().expect("server");
    assert_eq!(
        requests
            .iter()
            .filter(|request| request.contains("com.atproto.server.createSession"))
            .count(),
        1
    );
    assert!(requests[3]
        .to_lowercase()
        .contains("authorization: bearer new-jwt"));
    assert!(requests[4]
        .to_lowercase()
        .contains("authorization: bearer new-jwt"));
}
