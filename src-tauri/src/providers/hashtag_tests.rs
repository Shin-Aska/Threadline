use super::{
    bluesky::BlueskyProvider, mastodon::MastodonProvider, test_http::server, SocialProvider,
};
use crate::{hashtags::HashtagActivity, models::PreparedPost};
fn bluesky(url: String) -> BlueskyProvider {
    BlueskyProvider {
        app_password_session: Default::default(),
        capabilities: crate::accounts::mock_accounts()[0].capabilities.clone(),
        client: reqwest::Client::builder()
            .no_proxy()
            .build()
            .expect("client"),
        service_url: url,
        identifier: "fixture.invalid".into(),
        app_password: "fixture-only".into(),
        oauth: None,
    }
}
fn mastodon(url: String) -> MastodonProvider {
    MastodonProvider {
        capabilities: crate::accounts::mock_accounts()[1].capabilities.clone(),
        client: reqwest::Client::builder()
            .no_proxy()
            .build()
            .expect("client"),
        base_url: url,
        access_token: "fixture-only".into(),
    }
}
#[tokio::test]
async fn mastodon_uses_real_search_and_preserves_unknown_activity() {
    let (url, task) = server(vec![(
        200,
        r#"{"hashtags":[{"name":"Rust","history":[{"uses":"12"},{"uses":"5"}]},{"name":"RustLang"}]}"#,
    )]);
    let suggestions = mastodon(url).hashtags("Ru").await.expect("lookup");
    assert!(matches!(
        suggestions[0].activity,
        HashtagActivity::Mastodon { uses: 17, days: 2 }
    ));
    assert!(matches!(
        suggestions[1].activity,
        HashtagActivity::Unavailable
    ));
    let requests = task.join().expect("server");
    assert!(requests[0]
        .headers
        .starts_with("GET /api/v2/search?q=Ru&type=hashtags&limit=20 "));
    assert!(requests[0]
        .headers
        .to_lowercase()
        .contains("authorization: bearer fixture-only"));
}
#[tokio::test]
async fn bare_hashtag_fetches_mastodon_trends() {
    let (url, task) = server(vec![(
        200,
        r#"[{"name":"Caturday","history":[{"uses":"9"}]}]"#,
    )]);
    let suggestions = mastodon(url).hashtags("").await.expect("lookup");
    assert_eq!(suggestions[0].name, "Caturday");
    assert!(task.join().expect("server")[0]
        .headers
        .starts_with("GET /api/v1/trends/tags?limit=20 "));
}
#[tokio::test]
async fn bluesky_reuses_search_session_and_keeps_missing_count_unknown() {
    let (url, task) = server(vec![
        (
            200,
            r#"{"accessJwt":"fixture-jwt","did":"did:plc:fixture","handle":"fixture.invalid"}"#,
        ),
        (200, r#"{"hitsTotal":120,"posts":[]}"#),
        (200, r#"{"posts":[]}"#),
    ]);
    let provider = bluesky(url);
    let first = provider.hashtags("Rust").await.expect("lookup");
    let second = provider.hashtags("日本語").await.expect("lookup");
    assert!(matches!(
        first[0].activity,
        HashtagActivity::Bluesky { matches: Some(120) }
    ));
    assert!(matches!(
        second[0].activity,
        HashtagActivity::Bluesky { matches: None }
    ));
    let requests = task.join().expect("server");
    assert_eq!(requests.len(), 3);
    assert!(requests[1]
        .headers
        .starts_with("GET /xrpc/app.bsky.feed.searchPosts?q=%23Rust&tag=Rust&limit=1 "));
    assert!(requests[2]
        .headers
        .to_lowercase()
        .contains("authorization: bearer fixture-jwt"));
}
#[tokio::test]
async fn rejected_search_session_is_renewed_and_read_is_retried() {
    let (url, task) = server(vec![
        (
            200,
            r#"{"accessJwt":"old-jwt","did":"did:plc:fixture","handle":"fixture.invalid"}"#,
        ),
        (401, r#"{"error":"ExpiredToken"}"#),
        (
            200,
            r#"{"accessJwt":"new-jwt","did":"did:plc:fixture","handle":"fixture.invalid"}"#,
        ),
        (200, r#"{"hitsTotal":0,"posts":[]}"#),
    ]);
    let provider = bluesky(url);
    assert!(matches!(
        provider.hashtags("Rust").await.expect("retry")[0].activity,
        HashtagActivity::Bluesky { matches: Some(0) }
    ));
    let requests = task.join().expect("server");
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
async fn published_bluesky_record_contains_hashtag_facets() {
    let (url, task) = server(vec![
        (
            200,
            r#"{"accessJwt":"fixture-jwt","did":"did:plc:fixture","handle":"fixture.invalid"}"#,
        ),
        (200, r#"{"uri":"at://fixture/post/1","cid":"fixture-cid"}"#),
    ]);
    let text = "2/2 🧵 #Rust";
    bluesky(url)
        .publish(PreparedPost {
            text: text.into(),
            media: vec![],
        })
        .await
        .expect("publish");
    let requests = task.join().expect("server");
    let body: serde_json::Value = serde_json::from_slice(&requests[1].body).expect("record");
    assert_eq!(body["record"]["facets"][0]["index"]["byteStart"], 9);
    assert_eq!(body["record"]["facets"][0]["index"]["byteEnd"], 14);
    assert_eq!(
        body["record"]["facets"][0]["features"][0]["$type"],
        "app.bsky.richtext.facet#tag"
    );
    assert_eq!(body["record"]["facets"][0]["features"][0]["tag"], "Rust");
}
