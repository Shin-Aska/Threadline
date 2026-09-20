use super::test_http::server;
use super::{bluesky::BlueskyProvider, mastodon::MastodonProvider, SocialProvider};
use crate::models::{PreparedMedia, PreparedPost};
use std::sync::Arc;

fn post() -> PreparedPost {
    PreparedPost {
        text: "An image".into(),
        media: vec![PreparedMedia {
            mime_type: "image/png".into(),
            data: Arc::from(include_bytes!("../../icons/icon.png").as_slice()),
            alt_text: "Threadline logo 🧵".into(),
        }],
    }
}
fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .no_proxy()
        .build()
        .expect("client")
}
#[tokio::test]
async fn bluesky_uploads_bytes_and_embeds_alt_text() {
    let (url, task) = server(vec![
        (
            200,
            r#"{"accessJwt":"test-token","did":"did:plc:test","handle":"test.invalid"}"#,
        ),
        (
            200,
            r#"{"blob":{"$type":"blob","ref":{"$link":"test-blob"},"mimeType":"image/png","size":100}}"#,
        ),
        (200, r#"{"uri":"at://test/post/1","cid":"test-cid"}"#),
    ]);
    let provider = BlueskyProvider {
        app_password_session: Default::default(),
        capabilities: crate::accounts::mock_accounts()[0].capabilities.clone(),
        client: client(),
        service_url: url,
        identifier: "test.invalid".into(),
        app_password: "fixture-only".into(),
        oauth: None,
    };
    assert_eq!(
        provider.publish(post()).await.expect("publish").remote_id,
        "at://test/post/1"
    );
    let requests = task.join().expect("server");
    assert!(requests[1]
        .headers
        .starts_with("POST /xrpc/com.atproto.repo.uploadBlob "));
    assert!(requests[1]
        .headers
        .to_lowercase()
        .contains("authorization: bearer test-token"));
    assert!(requests[1]
        .headers
        .to_lowercase()
        .contains("content-type: image/png"));
    assert_eq!(requests[1].body, post().media[0].data.as_ref());
    let body: serde_json::Value = serde_json::from_slice(&requests[2].body).expect("record");
    assert_eq!(body["record"]["embed"]["$type"], "app.bsky.embed.images");
    assert_eq!(
        body["record"]["embed"]["images"][0]["alt"],
        "Threadline logo 🧵"
    );
    assert_eq!(
        body["record"]["embed"]["images"][0]["image"]["ref"]["$link"],
        "test-blob"
    );
}
#[tokio::test]
async fn mastodon_waits_for_processing_and_attaches_media_ids() {
    let (url, task) = server(vec![
        (202, r#"{"id":"media-1","url":null}"#),
        (206, ""),
        (
            200,
            r#"{"id":"media-1","url":"https://test.invalid/image.png"}"#,
        ),
        (200, r#"{"id":"post-1"}"#),
    ]);
    let provider = MastodonProvider {
        capabilities: crate::accounts::mock_accounts()[1].capabilities.clone(),
        client: client(),
        base_url: url,
        access_token: "test-token".into(),
    };
    assert_eq!(
        provider.publish(post()).await.expect("publish").remote_id,
        "post-1"
    );
    let requests = task.join().expect("server");
    assert!(requests[0].headers.starts_with("POST /api/v2/media "));
    assert!(requests[0]
        .headers
        .to_lowercase()
        .contains("authorization: bearer test-token"));
    let multipart = String::from_utf8_lossy(&requests[0].body);
    assert!(multipart.contains("name=\"description\"\r\n\r\nThreadline logo 🧵"));
    assert!(multipart.contains("Content-Type: image/png"));
    assert!(requests[0]
        .body
        .windows(post().media[0].data.len())
        .any(|bytes| bytes == post().media[0].data.as_ref()));
    assert!(requests[1]
        .headers
        .starts_with("GET /api/v1/media/media-1 "));
    assert!(requests[3].headers.starts_with("POST /api/v1/statuses "));
    assert!(String::from_utf8_lossy(&requests[3].body).contains("media_ids%5B%5D=media-1"));
}
#[tokio::test]
async fn rejected_upload_returns_failure_without_publishing_text() {
    let (url, task) = server(vec![(403, r#"{"error":"Missing media permission"}"#)]);
    let provider = MastodonProvider {
        capabilities: crate::accounts::mock_accounts()[1].capabilities.clone(),
        client: client(),
        base_url: url,
        access_token: "test-token".into(),
    };
    let error = provider
        .publish(post())
        .await
        .expect_err("upload must fail");
    assert!(error.to_string().contains("image upload"));
    assert_eq!(task.join().expect("server").len(), 1);
}
