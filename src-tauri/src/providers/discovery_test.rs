use super::{bluesky::BlueskyProvider, mastodon::MastodonProvider, SocialProvider};

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
async fn mastodon_discovery_fetches_native_topics_suggestions_and_statuses() {
    // Given each native Mastodon discovery surface at a real HTTP boundary.
    let tags = r##"[{"name":"Rust","history":[{"uses":"4"},{"uses":"6"}]}]"##;
    let suggestions = r##"[{"source":"global","account":{"id":"8","acct":"alice","display_name":"Alice","avatar":"https://social.test/a.png"}}]"##;
    let statuses = r##"[{"id":"42","url":"https://social.test/@alice/42","created_at":"2026-09-20T01:00:00Z","content":"<p>Popular</p>","account":{"id":"8","acct":"alice","display_name":"Alice"},"media_attachments":[],"replies_count":1,"reblogs_count":2,"favourites_count":3}]"##;
    let (url, server) =
        super::test_http::server(vec![(200, tags), (200, suggestions), (200, statuses)]);

    // When discovery is requested for one selected account.
    let result = mastodon(url)
        .discovery("account-1", "@me@social.test")
        .await
        .expect("discovery");

    // Then provider counts and post metrics are preserved without invention.
    assert_eq!(result["topics"][0]["postCount"], 10);
    assert_eq!(result["suggestedAccounts"][0]["id"], "8");
    assert_eq!(result["popularPosts"][0]["metrics"]["likes"], 3);
    let requests = server.join().expect("server");
    assert!(requests[0].headers.contains("/api/v1/trends/tags?limit=20"));
    assert!(requests[1].headers.contains("/api/v2/suggestions?limit=20"));
    assert!(requests[2]
        .headers
        .contains("/api/v1/trends/statuses?limit=20"));
}

#[tokio::test]
async fn bluesky_discovery_fetches_native_topics_and_actor_suggestions() {
    // Given authenticated native Bluesky topic and actor results.
    let session = r##"{"accessJwt":"jwt","did":"did:plc:me","handle":"me.test"}"##;
    let topics = r##"{"topics":[{"topic":"rustlang","displayName":"Rust","link":"https://bsky.app/search?q=rustlang"}],"suggested":[]}"##;
    let actors = r##"{"actors":[{"did":"did:plc:alice","handle":"alice.test","displayName":"Alice","avatar":"https://cdn.test/a.png"}]}"##;
    let (url, server) =
        super::test_http::server(vec![(200, session), (200, topics), (200, actors)]);

    // When discovery is requested for one selected account.
    let result = bluesky(url)
        .discovery("account-1", "@me.test")
        .await
        .expect("discovery");

    // Then exact provider topics and actors are normalized, with no fake posts.
    assert_eq!(result["topics"][0]["key"], "rustlang");
    assert_eq!(result["suggestedAccounts"][0]["id"], "did:plc:alice");
    assert_eq!(result["popularPosts"].as_array().map(Vec::len), Some(0));
    let requests = server.join().expect("server");
    assert!(requests[1]
        .headers
        .contains("/xrpc/app.bsky.unspecced.getTrendingTopics?limit=20"));
    assert!(requests[2]
        .headers
        .contains("/xrpc/app.bsky.actor.getSuggestions?limit=20"));
}
