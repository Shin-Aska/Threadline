use super::{
    bluesky::BlueskyProvider, mastodon::MastodonProvider, social::ProfileFeedKind, SocialProvider,
};

fn mastodon(base_url: String) -> MastodonProvider {
    MastodonProvider {
        capabilities: crate::accounts::mock_accounts()[1].capabilities.clone(),
        client: reqwest::Client::builder()
            .no_proxy()
            .build()
            .expect("client"),
        base_url,
        access_token: "token".into(),
    }
}

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

#[tokio::test]
async fn mastodon_profile_feed_normalizes_viewer_state_and_cursor() {
    // Given an account status response from a real HTTP boundary.
    let body = r#"[{"id":"42","url":"https://social.test/@alice/42","created_at":"2026-09-20T01:00:00Z","content":"<p>Hello</p>","account":{"id":"7","acct":"alice","display_name":"Alice","avatar":"https://social.test/a.png"},"media_attachments":[],"replies_count":2,"reblogs_count":3,"favourites_count":4,"favourited":true,"reblogged":false,"in_reply_to_id":null}]"#;
    let (url, server) = super::test_http::server(vec![(200, body)]);

    // When a later page of profile posts is requested.
    let page = mastodon(url)
        .profile_feed("7", ProfileFeedKind::Posts, Some("50"))
        .await
        .expect("profile feed");

    // Then the typed contract preserves native identity, state, and pagination.
    assert_eq!(page.posts[0].remote_id, "42");
    assert!(page.posts[0].viewer.liked);
    assert_eq!(page.cursor.as_deref(), Some("42"));
    let requests = server.join().expect("server");
    assert!(requests[0].headers.starts_with("GET /api/v1/accounts/7/statuses?limit=40&exclude_reblogs=true&exclude_replies=true&max_id=50 HTTP/1.1"));
}

#[tokio::test]
async fn bluesky_profile_feed_sends_cursor_and_normalizes_strong_reference() {
    // Given a session and author-feed response from a real HTTP boundary.
    let session = r#"{"accessJwt":"jwt","did":"did:plc:alice","handle":"alice.test"}"#;
    let feed = r#"{"cursor":"next","feed":[{"post":{"uri":"at://did:plc:alice/app.bsky.feed.post/3k","cid":"bafy-post","author":{"did":"did:plc:alice","handle":"alice.test","displayName":"Alice"},"record":{"text":"Hello","createdAt":"2026-09-20T01:00:00Z"},"replyCount":1,"repostCount":2,"likeCount":3,"viewer":{"like":"at://did:plc:me/app.bsky.feed.like/like1"}}}]}"#;
    let (url, server) = super::test_http::server(vec![(200, session), (200, feed)]);

    // When a profile feed page is requested with an opaque cursor.
    let page = bluesky(url)
        .profile_feed("did:plc:alice", ProfileFeedKind::Posts, Some("opaque"))
        .await
        .expect("profile feed");

    // Then the CID and viewer record URI are available for account-aware actions.
    assert_eq!(page.posts[0].remote_cid.as_deref(), Some("bafy-post"));
    assert_eq!(
        page.posts[0].viewer.like_uri.as_deref(),
        Some("at://did:plc:me/app.bsky.feed.like/like1")
    );
    assert_eq!(page.cursor.as_deref(), Some("next"));
    let requests = server.join().expect("server");
    assert!(requests[1].headers.contains("actor=did%3Aplc%3Aalice"));
    assert!(requests[1].headers.contains("cursor=opaque"));
}

#[tokio::test]
async fn mastodon_profile_feed_surfaces_provider_body_on_failure() {
    // Given a provider rejection with a useful JSON error body.
    let (url, server) = super::test_http::server(vec![(401, r#"{"error":"token expired"}"#)]);

    // When the profile feed is requested.
    let error = mastodon(url)
        .profile_feed("7", ProfileFeedKind::Posts, None)
        .await
        .expect_err("provider error");

    // Then the boundary reports status and sanitized provider detail.
    assert!(error.to_string().contains("401"));
    assert!(error.to_string().contains("token expired"));
    server.join().expect("server");
}

#[tokio::test]
async fn mastodon_following_uses_people_tags_and_lists_endpoints() {
    // Given native followed people, tags, and lists from one account.
    let own = r#"{"id":"7","acct":"me","display_name":"Me"}"#;
    let people = r#"[{"id":"8","acct":"alice","display_name":"Alice"}]"#;
    let tags = r#"[{"name":"rust"}]"#;
    let lists = r#"[{"id":"9","title":"Friends"}]"#;
    let (url, server) =
        super::test_http::server(vec![(200, own), (200, people), (200, tags), (200, lists)]);

    // When the followed-source page is loaded.
    let page = mastodon(url)
        .followed_sources_page(None)
        .await
        .expect("following sources");

    // Then each native membership is preserved with an actionable source type.
    assert_eq!(page.sources.len(), 3);
    assert_eq!(page.cursor.as_deref(), Some("8"));
    assert!(matches!(
        page.sources[0].source_type,
        super::social::SourceKind::Person
    ));
    assert!(matches!(
        page.sources[1].source_type,
        super::social::SourceKind::Tag
    ));
    assert!(matches!(
        page.sources[2].source_type,
        super::social::SourceKind::List
    ));
    let requests = server.join().expect("server");
    assert!(requests[1]
        .headers
        .contains("/api/v1/accounts/7/following?limit=80"));
    assert!(requests[2].headers.contains("/api/v1/followed_tags"));
    assert!(requests[3].headers.contains("/api/v1/lists"));
}

#[tokio::test]
async fn mastodon_notifications_preserve_remote_inbox_items() {
    // Given one native notification returned by the remote inbox.
    let notifications = r#"[{"id":"11","type":"mention","created_at":"2026-09-20T01:00:00Z","account":{"id":"8","acct":"alice","display_name":"Alice"},"status":null}]"#;
    let (url, server) = super::test_http::server(vec![(200, notifications)]);
    let provider = mastodon(url);

    // When notifications are loaded from the selected account.
    let page = provider.notifications(None).await.expect("notifications");

    // Then the inbox item remains present for the local read overlay.
    assert!(page.notifications[0].unread);
    assert!(matches!(
        page.notifications[0].kind,
        super::social::NotificationKind::Mention
    ));
    assert_eq!(server.join().expect("server").len(), 1);
}

#[tokio::test]
async fn bluesky_like_resolves_post_cid_before_creating_record() {
    // Given a trusted post view and a later create-record response.
    let session = r#"{"accessJwt":"jwt","did":"did:plc:me","handle":"me.test"}"#;
    let posts = r#"{"posts":[{"uri":"at://did:plc:alice/app.bsky.feed.post/3k","cid":"trusted-cid","author":{"did":"did:plc:alice","handle":"alice.test"},"record":{"text":"Hello","createdAt":"2026-09-20T01:00:00Z"}}]}"#;
    let created = r#"{"uri":"at://did:plc:me/app.bsky.feed.like/like1","cid":"like-cid"}"#;
    let (url, server) =
        super::test_http::server(vec![(200, session), (200, posts), (200, created)]);

    // When the selected account likes a post using only its native URI.
    let result = bluesky(url)
        .social_action(super::social::SocialAction::Like {
            post_id: "at://did:plc:alice/app.bsky.feed.post/3k".into(),
        })
        .await
        .expect("like");

    // Then the provider re-fetched and wrote the trusted strong-reference CID.
    assert!(result.viewer.as_ref().is_some_and(|viewer| viewer.liked));
    let requests = server.join().expect("server");
    let body: serde_json::Value = serde_json::from_slice(&requests[2].body).expect("record body");
    assert_eq!(body["record"]["subject"]["cid"], "trusted-cid");
    assert_eq!(body["collection"], "app.bsky.feed.like");
}
