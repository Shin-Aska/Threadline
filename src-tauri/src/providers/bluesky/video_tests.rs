use super::*;
use crate::{
    models::PreparedPost,
    providers::{bluesky::BlueskyProvider, test_http::server, SocialProvider},
};
use std::sync::Arc;

fn client() -> reqwest::Client {
    reqwest::Client::builder()
        .no_proxy()
        .build()
        .expect("client")
}

fn video() -> PreparedMedia {
    PreparedMedia {
        mime_type: "video/mp4".into(),
        data: Arc::from(b"fixture mp4 bytes".as_slice()),
        alt_text: "A demo".into(),
    }
}

#[tokio::test]
async fn uploads_after_live_limit_check_and_waits_for_completed_blob() {
    // Given live allowance, service authorization, and a processing job fixture.
    let (url, task) = server(vec![
        (200, r#"{"token":"limits-token"}"#),
        (
            200,
            r#"{"canUpload":true,"remainingDailyVideos":1,"remainingDailyBytes":1000}"#,
        ),
        (200, r#"{"token":"upload-token"}"#),
        (
            200,
            r#"{"jobStatus":{"jobId":"job-1","did":"did:plc:test","state":"JOB_STATE_ENCODING","progress":50}}"#,
        ),
        (
            200,
            r#"{"jobStatus":{"jobId":"job-1","did":"did:plc:test","state":"JOB_STATE_COMPLETED","progress":100,"blob":{"$type":"blob","ref":{"$link":"video-cid"},"mimeType":"video/mp4","size":17}}}"#,
        ),
    ]);

    // When one MP4 is uploaded through the video service.
    let blob = upload_video(
        &client(),
        &url,
        &url,
        "session-token",
        "did:plc:test",
        &video(),
    )
    .await
    .expect("video upload");

    // Then the processed blob is returned and the post record can safely use it.
    assert_eq!(blob["ref"]["$link"], "video-cid");
    let requests = task.join().expect("server");
    assert!(requests[0]
        .headers
        .contains("com.atproto.server.getServiceAuth"));
    assert!(requests[1]
        .headers
        .contains("app.bsky.video.getUploadLimits"));
    assert!(requests[3].headers.contains("app.bsky.video.uploadVideo"));
    assert_eq!(requests[3].body, video().data.as_ref());
    assert!(requests[4].headers.contains("app.bsky.video.getJobStatus"));
}

#[tokio::test]
async fn denied_upload_limit_stops_before_video_bytes_are_sent() {
    // Given an account that the live video service refuses.
    let (url, task) = server(vec![
        (200, r#"{"token":"limits-token"}"#),
        (
            200,
            r#"{"canUpload":false,"error":"upload_forbidden","message":"Email verification required"}"#,
        ),
    ]);

    // When video upload is requested.
    let error = upload_video(
        &client(),
        &url,
        &url,
        "session-token",
        "did:plc:test",
        &video(),
    )
    .await
    .expect_err("limit denial");

    // Then the precise service reason is surfaced without uploading or publishing.
    assert!(error.to_string().contains("Email verification required"));
    assert_eq!(task.join().expect("server").len(), 2);
}

#[tokio::test]
async fn processing_denial_never_falls_back_to_a_text_only_record() {
    // Given a post with text and video whose account cannot upload video.
    let (url, task) = server(vec![
        (
            200,
            r#"{"accessJwt":"session-token","did":"did:plc:test","handle":"test.invalid"}"#,
        ),
        (200, r#"{"token":"limits-token"}"#),
        (
            200,
            r#"{"canUpload":false,"message":"Video allowance exhausted"}"#,
        ),
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

    // When publication reaches the live video limit check.
    let error = provider
        .publish(PreparedPost {
            text: "Text must not escape alone".into(),
            media: vec![video()],
        })
        .await
        .expect_err("video denial");

    // Then publication fails and no createRecord request is made.
    assert!(error.to_string().contains("Video allowance exhausted"));
    let requests = task.join().expect("server");
    assert_eq!(requests.len(), 3);
    assert!(requests
        .iter()
        .all(|request| !request.headers.contains("com.atproto.repo.createRecord")));
}
