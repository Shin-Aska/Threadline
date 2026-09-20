use super::{
    callback::{parse_callback_target, CallbackPayload, LoopbackCallback},
    config::{BlueskyClientMode, HostedClientDocument},
    coordinator::OAuthCoordinator,
};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpStream,
};
use tokio_util::sync::CancellationToken;

#[test]
fn callback_accepts_code_when_path_and_state_match() {
    // Given an exact callback path and an unpredictable expected state.
    let target = "/oauth/callback?code=secret-code&state=expected";

    // When the callback target is parsed at the listener boundary.
    let parsed = parse_callback_target(target, "/oauth/callback", "expected");

    // Then the authorization response is accepted without exposing it in an error.
    assert_eq!(
        parsed,
        Ok(CallbackPayload {
            code: "secret-code".into(),
            state: "expected".into(),
            issuer: None,
        })
    );
}

#[test]
fn callback_rejects_forged_state() {
    // Given a callback carrying a state from another browser flow.
    let target = "/oauth/callback?code=secret-code&state=forged";

    // When the listener parses it against the active flow state.
    let parsed = parse_callback_target(target, "/oauth/callback", "expected");

    // Then it is rejected before the code can be exchanged.
    assert!(matches!(parsed, Err(super::OAuthError::StateMismatch)));
}

#[test]
fn callback_rejects_another_path() {
    // Given a request sent to a different local path.
    let target = "/not-the-callback?code=secret-code&state=expected";

    // When the listener parses it.
    let parsed = parse_callback_target(target, "/oauth/callback", "expected");

    // Then the request cannot enter the OAuth exchange.
    assert!(matches!(parsed, Err(super::OAuthError::CallbackPath)));
}

#[test]
fn hosted_metadata_requires_native_dpop_and_atproto_scope() {
    // Given metadata that attempts an ordinary bearer OAuth client.
    let document = HostedClientDocument {
        client_id: "https://example.com/oauth-client-metadata.json".into(),
        application_type: "native".into(),
        grant_types: vec!["authorization_code".into(), "refresh_token".into()],
        scope: "atproto transition:generic".into(),
        response_types: vec!["code".into()],
        redirect_uris: vec!["com.example:/oauth/callback".into()],
        dpop_bound_access_tokens: false,
        token_endpoint_auth_method: Some("none".into()),
    };

    // When the hosted document is parsed into an executable client mode.
    let parsed = BlueskyClientMode::from_hosted_document(
        "https://example.com/oauth-client-metadata.json",
        document,
    );

    // Then the unsafe client is rejected.
    assert!(matches!(parsed, Err(super::OAuthError::InvalidMetadata(_))));
}

#[test]
fn hosted_metadata_requires_reverse_domain_native_redirect() {
    // Given metadata whose custom callback scheme cannot belong to the client host.
    let document = HostedClientDocument {
        client_id: "https://example.com/oauth-client-metadata.json".into(),
        application_type: "native".into(),
        grant_types: vec!["authorization_code".into(), "refresh_token".into()],
        scope: "atproto transition:generic".into(),
        response_types: vec!["code".into()],
        redirect_uris: vec!["social.threadline:/oauth/callback".into()],
        dpop_bound_access_tokens: true,
        token_endpoint_auth_method: Some("none".into()),
    };

    // When the redirect ownership rule is checked.
    let parsed = BlueskyClientMode::from_hosted_document(
        "https://example.com/oauth-client-metadata.json",
        document,
    );

    // Then a callback the app cannot prove belongs to that metadata host is rejected.
    assert!(matches!(parsed, Err(super::OAuthError::InvalidMetadata(_))));
}

#[test]
fn hosted_metadata_requires_service_auth_scope_for_video() {
    // Given otherwise valid native metadata without the transitional service-auth scope.
    let document = HostedClientDocument {
        client_id: "https://example.com/oauth-client-metadata.json".into(),
        application_type: "native".into(),
        grant_types: vec!["authorization_code".into(), "refresh_token".into()],
        scope: "atproto".into(),
        response_types: vec!["code".into()],
        redirect_uris: vec!["com.example:/oauth/callback".into()],
        dpop_bound_access_tokens: true,
        token_endpoint_auth_method: Some("none".into()),
    };

    // When Threadline validates metadata for its video-capable publishing client.
    let parsed = BlueskyClientMode::from_hosted_document(
        "https://example.com/oauth-client-metadata.json",
        document,
    );

    // Then it rejects metadata that cannot authorize getServiceAuth.
    assert!(matches!(parsed, Err(super::OAuthError::InvalidMetadata(_))));
}

#[tokio::test]
async fn loopback_driver_returns_callback_and_closes_listener() {
    // Given an ephemeral loopback listener for one active OAuth flow.
    let callback = LoopbackCallback::bind("/oauth/callback")
        .await
        .expect("bind callback");
    let redirect = callback.redirect_uri().expect("redirect URI");
    let wait = tokio::spawn(callback.wait(
        Some("expected"),
        std::time::Duration::from_secs(2),
        CancellationToken::new(),
    ));

    // When a browser-shaped request reaches the exact callback URI.
    let url = url::Url::parse(&redirect).expect("parse redirect");
    let mut stream = TcpStream::connect(("127.0.0.1", url.port().expect("port")))
        .await
        .expect("connect callback");
    stream
        .write_all(b"GET /oauth/callback?code=local-code&state=expected HTTP/1.1\r\nHost: 127.0.0.1\r\n\r\n")
        .await
        .expect("write callback");
    let mut response = Vec::new();
    stream
        .read_to_end(&mut response)
        .await
        .expect("read response");
    let payload = wait.await.expect("callback task").expect("valid callback");

    // Then the code is delivered once and the listener is no longer reachable.
    assert_eq!(payload.code, "local-code");
    assert!(TcpStream::connect(("127.0.0.1", url.port().expect("port")))
        .await
        .is_err());
}

#[tokio::test]
async fn coordinator_cancellation_reaches_active_flow() {
    // Given an active native OAuth flow registered under a renderer-generated UUID.
    let coordinator = OAuthCoordinator::default();
    let flow_id = uuid::Uuid::new_v4().to_string();
    let flow = coordinator.start(&flow_id, false).expect("start flow");

    // When the cancel command targets that flow.
    coordinator.cancel(&flow_id).expect("cancel flow");

    // Then the native login cancellation token resolves without waiting for its timeout.
    tokio::time::timeout(
        std::time::Duration::from_millis(50),
        flow.cancel.cancelled(),
    )
    .await
    .expect("cancellation propagated");
}
