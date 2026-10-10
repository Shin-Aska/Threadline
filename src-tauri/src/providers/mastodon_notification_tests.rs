use crate::providers::{mastodon::MastodonProvider, SocialProvider};

fn mastodon(base_url: String) -> MastodonProvider {
    MastodonProvider {
        capabilities: crate::accounts::mock_accounts()[1].capabilities.clone(),
        client: reqwest::Client::builder()
            .no_proxy()
            .build()
            .expect("client"),
        base_url,
        access_token: "test-token".into(),
    }
}

#[tokio::test]
async fn mastodon_notifications_count_only_events_after_remote_read_marker() {
    // Given 40 inbox events and a remote marker indicating 38 were read elsewhere.
    let notifications = (1..=40)
        .rev()
        .map(|id| {
            serde_json::json!({
                "id": id.to_string(), "type": "follow", "created_at": "2026-10-10T01:00:00Z",
                "account": { "id": "8", "acct": "alice" }, "status": null
            })
        })
        .collect::<Vec<_>>();
    let notifications = Box::leak(
        serde_json::to_string(&notifications)
            .expect("fixture")
            .into_boxed_str(),
    );
    let (url, server) = crate::providers::test_http::server(vec![
        (200, notifications),
        (
            200,
            r#"{"notifications":{"last_read_id":"38","version":1}}"#,
        ),
    ]);

    // When the provider loads the full notification page.
    let page = mastodon(url)
        .notifications(None)
        .await
        .expect("notifications");

    // Then only the two new events are unread; read events remain in the list.
    assert_eq!(page.notifications.len(), 40);
    assert_eq!(
        page.notifications
            .iter()
            .filter(|item| item.unread)
            .map(|item| item.id.as_str())
            .collect::<Vec<_>>(),
        ["40", "39"]
    );
    let requests = server.join().expect("server");
    assert!(requests[1]
        .headers
        .contains("/api/v1/markers?timeline%5B%5D=notifications"));
}

#[tokio::test]
async fn mastodon_notification_ids_are_compared_numerically_across_digit_lengths() {
    // Given a paginated event with ID 10 and a remote marker at ID 9.
    let notifications = r#"[{"id":"10","type":"follow","created_at":"2026-10-10T01:00:00Z","account":{"id":"8","acct":"alice"},"status":null}]"#;
    let (url, server) = crate::providers::test_http::server(vec![
        (200, notifications),
        (200, r#"{"notifications":{"last_read_id":"9"}}"#),
    ]);

    // When the account loads an older page.
    let page = mastodon(url).notifications(Some("11")).await.expect("page");

    // Then numeric ordering recognizes ID 10 as unread after ID 9.
    assert!(page.notifications[0].unread);
    let requests = server.join().expect("server");
    assert!(requests[0].headers.contains("max_id=11"));
}
