use super::{bluesky::BlueskyProvider, SocialProvider};
use std::{
    io::{BufRead, BufReader, Write},
    net::TcpListener,
    time::Duration,
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

fn actor_json() -> serde_json::Value {
    serde_json::json!({"did":"did:plc:actor","handle":"actor.test","displayName":"Actor"})
}

fn post_json(uri: &str) -> serde_json::Value {
    serde_json::json!({
        "uri": uri,
        "cid": format!("cid-{}", uri.rsplit('/').next().unwrap_or_default()),
        "author": actor_json(),
        "record": {"text": uri, "createdAt":"2026-09-20T01:00:00Z"}
    })
}

fn notification_json(index: usize, subject: &str) -> serde_json::Value {
    serde_json::json!({
        "uri": format!("at://did:plc:actor/app.bsky.feed.like/{index}"),
        "cid": format!("notification-{index}"),
        "author": actor_json(),
        "reason": "like",
        "reasonSubject": subject,
        "record": {"text":"", "createdAt":"2026-09-20T01:00:00Z"},
        "isRead": false,
        "indexedAt": "2026-09-20T01:00:00Z"
    })
}

fn notification_server(
    notifications: serde_json::Value,
) -> (String, std::thread::JoinHandle<Vec<String>>) {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind");
    listener.set_nonblocking(true).expect("nonblocking");
    let url = format!("http://{}", listener.local_addr().expect("address"));
    let task = std::thread::spawn(move || {
        let mut targets = Vec::new();
        let mut received_request = false;
        let mut idle_cycles = 0;
        loop {
            let (socket, _) = match listener.accept() {
                Ok(connection) => {
                    idle_cycles = 0;
                    connection
                }
                Err(error) if error.kind() == std::io::ErrorKind::WouldBlock => {
                    if received_request {
                        idle_cycles += 1;
                    }
                    if idle_cycles >= 10 {
                        break;
                    }
                    if received_request {
                        std::thread::sleep(Duration::from_millis(100));
                    } else {
                        std::thread::sleep(Duration::from_millis(10));
                    }
                    continue;
                }
                Err(error) => panic!("accept: {error}"),
            };
            received_request = true;
            let mut reader = BufReader::new(socket);
            let mut request_line = String::new();
            reader.read_line(&mut request_line).expect("request line");
            let target = request_line
                .split_whitespace()
                .nth(1)
                .expect("request target")
                .to_owned();
            let body = if target.contains("com.atproto.server.createSession") {
                serde_json::json!({
                    "accessJwt":"jwt",
                    "did":"did:plc:alice",
                    "handle":"alice.test"
                })
            } else if target.contains("app.bsky.notification.listNotifications") {
                notifications.clone()
            } else if target.contains("app.bsky.feed.getPosts") {
                let parsed =
                    url::Url::parse(&format!("http://localhost{target}")).expect("request URL");
                let posts = parsed
                    .query_pairs()
                    .filter(|(name, _)| name == "uris")
                    .map(|(_, uri)| post_json(&uri))
                    .collect::<Vec<_>>();
                serde_json::json!({"posts": posts})
            } else {
                panic!("unexpected request: {target}");
            };
            targets.push(target);
            let body = serde_json::to_string(&body).expect("response body");
            write!(
                reader.get_mut(),
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                body.len()
            )
            .expect("response");
        }
        targets
    });
    (url, task)
}

#[tokio::test]
async fn bluesky_notifications_batch_and_deduplicate_subject_lookups() {
    // Given 27 notifications referring to 26 distinct post subjects.
    let subjects = (0..26)
        .map(|index| format!("at://did:plc:author/app.bsky.feed.post/{index}"))
        .collect::<Vec<_>>();
    let mut notifications = subjects
        .iter()
        .enumerate()
        .map(|(index, subject)| notification_json(index, subject))
        .collect::<Vec<_>>();
    notifications.push(notification_json(26, &subjects[0]));
    let response = serde_json::json!({"cursor":"next", "notifications":notifications});
    let (url, server) = notification_server(response);

    // When the notification page is loaded.
    let page = bluesky(url)
        .notifications(None)
        .await
        .expect("notifications");

    // Then subjects are deduplicated into at most 25 URIs per getPosts call.
    assert_eq!(page.notifications.len(), 27);
    assert_eq!(
        page.notifications[0].id,
        "at://did:plc:actor/app.bsky.feed.like/0"
    );
    assert_eq!(
        page.notifications[26]
            .post
            .as_ref()
            .map(|post| post.remote_id.as_str()),
        page.notifications[0]
            .post
            .as_ref()
            .map(|post| post.remote_id.as_str())
    );
    let targets = server.join().expect("server");
    let post_targets = targets
        .iter()
        .filter(|target| target.contains("app.bsky.feed.getPosts"))
        .collect::<Vec<_>>();
    assert_eq!(post_targets.len(), 2);
    assert_eq!(
        post_targets[0].matches("uris=").count(),
        25,
        "first getPosts batch"
    );
    assert_eq!(
        post_targets[1].matches("uris=").count(),
        1,
        "second getPosts batch"
    );
    assert_eq!(targets.len(), 4, "session and XRPC HTTP requests");
}

#[tokio::test]
async fn bluesky_notification_survives_missing_related_post() {
    // Given a notification whose related post is absent from getPosts.
    let session = r#"{"accessJwt":"jwt","did":"did:plc:alice","handle":"alice.test"}"#;
    let notifications = serde_json::json!({
        "notifications": [notification_json(
            0,
            "at://did:plc:author/app.bsky.feed.post/missing"
        )]
    });
    let notifications = serde_json::to_string(&notifications).expect("notifications fixture");
    let (url, server) = super::test_http::server(vec![
        (200, session),
        (200, Box::leak(notifications.into_boxed_str())),
        (200, r#"{"posts":[]}"#),
    ]);

    // When the notification page is loaded.
    let page = bluesky(url)
        .notifications(None)
        .await
        .expect("notifications");

    // Then the notification remains present without a fabricated post.
    assert_eq!(page.notifications.len(), 1);
    assert!(page.notifications[0].post.is_none());
    assert_eq!(server.join().expect("server").len(), 3);
}

#[tokio::test]
async fn bluesky_notification_survives_related_post_fetch_failure() {
    // Given getPosts rejects a notification's related-post lookup.
    let session = r#"{"accessJwt":"jwt","did":"did:plc:alice","handle":"alice.test"}"#;
    let notifications = serde_json::json!({
        "notifications": [notification_json(
            0,
            "at://did:plc:author/app.bsky.feed.post/unavailable"
        )]
    });
    let notifications = serde_json::to_string(&notifications).expect("notifications fixture");
    let (url, server) = super::test_http::server(vec![
        (200, session),
        (200, Box::leak(notifications.into_boxed_str())),
        (503, r#"{"error":"Unavailable"}"#),
    ]);

    // When the notification page is loaded.
    let page = bluesky(url)
        .notifications(None)
        .await
        .expect("notifications");

    // Then the notification remains present without a related post.
    assert_eq!(page.notifications.len(), 1);
    assert!(page.notifications[0].post.is_none());
    assert_eq!(server.join().expect("server").len(), 3);
}
