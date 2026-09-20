//! Bluesky OAuth login, callback validation, and refresh-session persistence.
//!
//! Localhost login accepts a loopback callback; packaged login accepts a validated deep link.
//! The restored runtime persists the provider-managed session after authenticated requests so
//! token refresh remains inside the OAuth boundary rather than the app-password cache.

use super::{
    browser::{BrowserOpener, SystemBrowser},
    callback::{CallbackPayload, LoopbackCallback},
    config::{validate_hosted_metadata_url, BlueskyClientMode, HostedClientDocument},
    network::pinned_client,
    OAuthError, DEFAULT_LOGIN_TIMEOUT,
};
use crate::credentials::CredentialStore;
use atrium_api::{agent::SessionManager, types::string::Did};
use atrium_common::store::Store;
use atrium_identity::{
    did::{CommonDidResolver, CommonDidResolverConfig, DEFAULT_PLC_DIRECTORY_URL},
    handle::{AtprotoHandleResolver, AtprotoHandleResolverConfig, DnsTxtResolver},
};
use atrium_oauth::{
    store::{
        session::{MemorySessionStore, Session},
        state::MemoryStateStore,
    },
    AtprotoClientMetadata, AtprotoLocalhostClientMetadata, AuthorizeOptions, CallbackParams,
    KnownScope, OAuthClient, OAuthClientConfig, OAuthResolverConfig, OAuthSession, Scope,
};
use atrium_xrpc::{
    http::{Request, Response},
    HttpClient, XrpcClient,
};
use serde::{Deserialize, Serialize};
use std::{error::Error, sync::Arc, time::Duration};
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;
use url::Url;

const CALLBACK_PATH: &str = "/oauth/callback";
const MAX_HTTP_BODY_BYTES: usize = 2 * 1024 * 1024;

#[derive(Debug, Clone)]
pub struct BlueskyLoginRequest {
    pub identifier: String,
    pub metadata_url: Option<String>,
    pub timeout: Duration,
}

impl BlueskyLoginRequest {
    pub fn new(identifier: String, metadata_url: Option<String>) -> Self {
        Self {
            identifier,
            metadata_url,
            timeout: DEFAULT_LOGIN_TIMEOUT,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BlueskyOAuthCredential {
    pub(crate) subject: Did,
    pub(crate) session: Session,
    pub(crate) client: StoredClientConfig,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(tag = "mode", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum StoredClientConfig {
    Localhost { redirect_uri: String },
    Hosted { metadata: HostedClientDocument },
}

impl BlueskyOAuthCredential {
    pub fn did(&self) -> &str {
        self.subject.as_ref()
    }

    pub async fn restore(self) -> Result<BlueskyOAuthRuntime, OAuthError> {
        self.restore_with_persistence(None).await
    }

    pub(crate) async fn restore_with_persistence(
        self,
        persistence: Option<Arc<dyn CredentialStore>>,
    ) -> Result<BlueskyOAuthRuntime, OAuthError> {
        let mode = match &self.client {
            StoredClientConfig::Localhost { redirect_uri } => {
                BlueskyClientMode::localhost(redirect_uri.clone())
            }
            StoredClientConfig::Hosted { metadata } => {
                BlueskyClientMode::from_hosted_document(&metadata.client_id, metadata.clone())?
            }
        };
        let (client, sessions) = build_client(mode)?;
        let client_config = self.client.clone();
        sessions
            .set(self.subject.clone(), self.session)
            .await
            .map_err(|error| OAuthError::Provider(error.to_string()))?;
        let session = client
            .restore(&self.subject)
            .await
            .map_err(|error| OAuthError::Provider(error.to_string()))?;
        Ok(BlueskyOAuthRuntime {
            session,
            subject: self.subject,
            sessions,
            client_config,
            persistence,
        })
    }
}

pub async fn login_localhost(
    request: BlueskyLoginRequest,
    cancel: CancellationToken,
) -> Result<BlueskyOAuthCredential, OAuthError> {
    login_localhost_with_browser(request, cancel, &SystemBrowser).await
}

pub async fn login_localhost_with_browser(
    request: BlueskyLoginRequest,
    cancel: CancellationToken,
    browser: &dyn BrowserOpener,
) -> Result<BlueskyOAuthCredential, OAuthError> {
    if request.metadata_url.is_some() {
        return Err(OAuthError::Configuration(
            "hosted metadata requires the desktop deep-link callback".into(),
        ));
    }
    let callback = LoopbackCallback::bind(CALLBACK_PATH).await?;
    let redirect_uri = callback.redirect_uri()?;
    let mode = BlueskyClientMode::localhost(redirect_uri.clone());
    let (client, sessions) = build_client(mode)?;
    let authorize_url = authorize(&client, &request.identifier, &redirect_uri).await?;
    browser.open(&authorize_url)?;
    let payload = callback.wait(None, request.timeout, cancel).await?;
    finish(
        client,
        sessions,
        payload,
        StoredClientConfig::Localhost { redirect_uri },
    )
    .await
}

pub async fn login_hosted(
    request: BlueskyLoginRequest,
    callback: oneshot::Receiver<String>,
    cancel: CancellationToken,
) -> Result<BlueskyOAuthCredential, OAuthError> {
    login_hosted_with_browser(request, callback, cancel, &SystemBrowser).await
}

pub async fn login_hosted_with_browser(
    request: BlueskyLoginRequest,
    callback: oneshot::Receiver<String>,
    cancel: CancellationToken,
    browser: &dyn BrowserOpener,
) -> Result<BlueskyOAuthCredential, OAuthError> {
    let metadata_url = request.metadata_url.as_deref().ok_or_else(|| {
        OAuthError::Configuration(
            "a public HTTPS Bluesky client metadata URL is required for packaged login".into(),
        )
    })?;
    let metadata = fetch_hosted_metadata(metadata_url).await?;
    let mode = BlueskyClientMode::from_hosted_document(metadata_url, metadata.clone())?;
    let redirect_uri =
        metadata.redirect_uris.first().cloned().ok_or_else(|| {
            OAuthError::InvalidMetadata("a native redirect URI is required".into())
        })?;
    let (client, sessions) = build_client(mode)?;
    let authorize_url = authorize(&client, &request.identifier, &redirect_uri).await?;
    browser.open(&authorize_url)?;
    let callback_url = tokio::select! {
        _ = cancel.cancelled() => return Err(OAuthError::Cancelled),
        result = tokio::time::timeout(request.timeout, callback) => {
            result.map_err(|_| OAuthError::Timeout)?
                .map_err(|_| OAuthError::Cancelled)?
        }
    };
    let payload = parse_deep_link(&callback_url, &redirect_uri)?;
    finish(
        client,
        sessions,
        payload,
        StoredClientConfig::Hosted { metadata },
    )
    .await
}

async fn fetch_hosted_metadata(url: &str) -> Result<HostedClientDocument, OAuthError> {
    let parsed = validate_hosted_metadata_url(url)?;
    let client = pinned_client(&parsed).await?;
    let response = client.get(parsed).send().await.map_err(provider_request)?;
    if response.status() != reqwest::StatusCode::OK {
        return Err(OAuthError::InvalidMetadata(format!(
            "client metadata returned {}",
            response.status()
        )));
    }
    let content_type = response
        .headers()
        .get(reqwest::header::CONTENT_TYPE)
        .and_then(|value| value.to_str().ok())
        .unwrap_or_default();
    if !content_type
        .split(';')
        .next()
        .is_some_and(|value| value.trim() == "application/json")
    {
        return Err(OAuthError::InvalidMetadata(
            "client metadata must use application/json".into(),
        ));
    }
    response
        .json()
        .await
        .map_err(|_| OAuthError::InvalidMetadata("client metadata JSON is invalid".into()))
}

fn parse_deep_link(raw: &str, expected_redirect: &str) -> Result<CallbackPayload, OAuthError> {
    let callback = Url::parse(raw)
        .map_err(|_| OAuthError::InvalidResponse("malformed native callback".into()))?;
    let expected = Url::parse(expected_redirect)
        .map_err(|_| OAuthError::InvalidMetadata("native redirect URI is malformed".into()))?;
    if callback.scheme() != expected.scheme()
        || callback.host_str() != expected.host_str()
        || callback.port_or_known_default() != expected.port_or_known_default()
        || callback.path() != expected.path()
        || callback.fragment().is_some()
    {
        return Err(OAuthError::CallbackPath);
    }
    #[derive(Deserialize)]
    struct Query {
        code: Option<String>,
        state: Option<String>,
        iss: Option<String>,
        error: Option<String>,
    }
    let query: Query = serde_html_form::from_str(callback.query().unwrap_or_default())
        .map_err(|_| OAuthError::InvalidResponse("malformed native callback query".into()))?;
    if query.error.is_some() {
        return Err(OAuthError::ProviderDenied);
    }
    Ok(CallbackPayload {
        code: query
            .code
            .ok_or_else(|| OAuthError::InvalidResponse("authorization code is missing".into()))?,
        state: query
            .state
            .ok_or_else(|| OAuthError::InvalidResponse("callback state is missing".into()))?,
        issuer: query.iss,
    })
}

type DidResolver = CommonDidResolver<HardenedHttpClient>;
type HandleResolver = AtprotoHandleResolver<NoDnsResolver, HardenedHttpClient>;
type Client = OAuthClient<
    MemoryStateStore,
    MemorySessionStore,
    DidResolver,
    HandleResolver,
    HardenedHttpClient,
>;
type SessionClient =
    OAuthSession<HardenedHttpClient, DidResolver, HandleResolver, MemorySessionStore>;

pub struct BlueskyOAuthRuntime {
    session: SessionClient,
    subject: Did,
    sessions: MemorySessionStore,
    client_config: StoredClientConfig,
    persistence: Option<Arc<dyn CredentialStore>>,
}

impl BlueskyOAuthRuntime {
    pub fn subject(&self) -> &str {
        self.subject.as_ref()
    }

    pub fn service_url(&self) -> String {
        self.session.base_uri()
    }

    pub async fn get_json<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        query: &[(&str, &str)],
    ) -> Result<T, OAuthError> {
        self.request_json(
            atrium_xrpc::http::Method::GET,
            path,
            query,
            Vec::new(),
            None,
        )
        .await
    }

    pub async fn post_json<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        body: &serde_json::Value,
    ) -> Result<T, OAuthError> {
        let bytes = serde_json::to_vec(body)
            .map_err(|error| OAuthError::InvalidResponse(error.to_string()))?;
        self.request_json(
            atrium_xrpc::http::Method::POST,
            path,
            &[],
            bytes,
            Some("application/json"),
        )
        .await
    }

    pub async fn post_bytes<T: serde::de::DeserializeOwned>(
        &self,
        path: &str,
        mime_type: &str,
        body: Vec<u8>,
    ) -> Result<T, OAuthError> {
        self.request_json(
            atrium_xrpc::http::Method::POST,
            path,
            &[],
            body,
            Some(mime_type),
        )
        .await
    }

    async fn request_json<T: serde::de::DeserializeOwned>(
        &self,
        method: atrium_xrpc::http::Method,
        path: &str,
        query: &[(&str, &str)],
        body: Vec<u8>,
        content_type: Option<&str>,
    ) -> Result<T, OAuthError> {
        let mut url = Url::parse(&self.session.base_uri())
            .and_then(|base| base.join(&format!("xrpc/{path}")))
            .map_err(|_| OAuthError::Configuration("Bluesky PDS URL is invalid".into()))?;
        url.query_pairs_mut().extend_pairs(query.iter().copied());
        let mut builder = Request::builder().method(method).uri(url.as_str());
        if let Some(content_type) = content_type {
            builder = builder.header(atrium_xrpc::http::header::CONTENT_TYPE, content_type);
        }
        let response =
            self.session
                .send_http(builder.body(body).map_err(|_| {
                    OAuthError::InvalidResponse("Bluesky request is invalid".into())
                })?)
                .await;
        self.persist_session().await?;
        let response = response.map_err(|error| OAuthError::Provider(error.to_string()))?;
        if !response.status().is_success() {
            return Err(OAuthError::Provider(format!(
                "Bluesky returned {}",
                response.status()
            )));
        }
        serde_json::from_slice(response.body())
            .map_err(|_| OAuthError::InvalidResponse("Bluesky returned invalid JSON".into()))
    }

    async fn persist_session(&self) -> Result<(), OAuthError> {
        let Some(store) = &self.persistence else {
            return Ok(());
        };
        let session = self
            .sessions
            .get(&self.subject)
            .await
            .map_err(|error| OAuthError::Provider(error.to_string()))?
            .ok_or_else(|| {
                OAuthError::InvalidResponse("Bluesky OAuth session was not retained".into())
            })?;
        let encoded = serde_json::to_string(&BlueskyOAuthCredential {
            subject: self.subject.clone(),
            session,
            client: self.client_config.clone(),
        })
        .map_err(|error| OAuthError::InvalidResponse(error.to_string()))?;
        let subject: &str = self.subject.as_ref();
        store
            .set(&format!("bsky-{subject}"), &encoded)
            .map_err(|error| OAuthError::Provider(error.to_string()))
    }
}

impl HttpClient for BlueskyOAuthRuntime {
    async fn send_http(
        &self,
        request: Request<Vec<u8>>,
    ) -> Result<Response<Vec<u8>>, Box<dyn Error + Send + Sync + 'static>> {
        let response = self.session.send_http(request).await;
        self.persist_session().await?;
        response
    }
}

fn build_client(mode: BlueskyClientMode) -> Result<(Client, MemorySessionStore), OAuthError> {
    match mode {
        BlueskyClientMode::Localhost(metadata) => make_local_client(metadata),
        BlueskyClientMode::Hosted(metadata) => make_hosted_client(metadata),
    }
}

fn make_local_client(
    metadata: AtprotoLocalhostClientMetadata,
) -> Result<(Client, MemorySessionStore), OAuthError> {
    let http = HardenedHttpClient::new()?;
    let shared = Arc::new(http.clone());
    let sessions = MemorySessionStore::default();
    let client = OAuthClient::new(OAuthClientConfig {
        client_metadata: metadata,
        keys: None,
        state_store: MemoryStateStore::default(),
        session_store: sessions.clone(),
        resolver: OAuthResolverConfig {
            did_resolver: CommonDidResolver::new(CommonDidResolverConfig {
                plc_directory_url: DEFAULT_PLC_DIRECTORY_URL.into(),
                http_client: Arc::clone(&shared),
            }),
            handle_resolver: AtprotoHandleResolver::new(AtprotoHandleResolverConfig {
                dns_txt_resolver: NoDnsResolver,
                http_client: shared,
            }),
            authorization_server_metadata: Default::default(),
            protected_resource_metadata: Default::default(),
        },
        http_client: http,
    })
    .map_err(|error| OAuthError::Configuration(error.to_string()))?;
    Ok((client, sessions))
}

fn make_hosted_client(
    metadata: AtprotoClientMetadata,
) -> Result<(Client, MemorySessionStore), OAuthError> {
    let http = HardenedHttpClient::new()?;
    let shared = Arc::new(http.clone());
    let sessions = MemorySessionStore::default();
    let client = OAuthClient::new(OAuthClientConfig {
        client_metadata: metadata,
        keys: None,
        state_store: MemoryStateStore::default(),
        session_store: sessions.clone(),
        resolver: OAuthResolverConfig {
            did_resolver: CommonDidResolver::new(CommonDidResolverConfig {
                plc_directory_url: DEFAULT_PLC_DIRECTORY_URL.into(),
                http_client: Arc::clone(&shared),
            }),
            handle_resolver: AtprotoHandleResolver::new(AtprotoHandleResolverConfig {
                dns_txt_resolver: NoDnsResolver,
                http_client: shared,
            }),
            authorization_server_metadata: Default::default(),
            protected_resource_metadata: Default::default(),
        },
        http_client: http,
    })
    .map_err(|error| OAuthError::Configuration(error.to_string()))?;
    Ok((client, sessions))
}

async fn authorize(
    client: &Client,
    identifier: &str,
    redirect_uri: &str,
) -> Result<String, OAuthError> {
    if identifier.trim().is_empty() {
        return Err(OAuthError::Configuration(
            "Bluesky handle or DID is required".into(),
        ));
    }
    client
        .authorize(
            identifier.trim(),
            AuthorizeOptions {
                redirect_uri: Some(redirect_uri.into()),
                scopes: vec![
                    Scope::Known(KnownScope::Atproto),
                    Scope::Known(KnownScope::TransitionGeneric),
                ],
                prompt: None,
                state: None,
            },
        )
        .await
        .map_err(|error| OAuthError::Provider(error.to_string()))
}

async fn finish(
    client: Client,
    sessions: MemorySessionStore,
    payload: CallbackPayload,
    stored_client: StoredClientConfig,
) -> Result<BlueskyOAuthCredential, OAuthError> {
    let callback = tokio::spawn(async move {
        client
            .callback(CallbackParams {
                code: payload.code,
                state: Some(payload.state),
                iss: payload.issuer,
            })
            .await
    })
    .await
    .map_err(|_| OAuthError::Provider("Bluesky rejected the token exchange".into()))?
    .map_err(|error| OAuthError::Provider(error.to_string()))?;
    let subject = callback.0.did().await.ok_or_else(|| {
        OAuthError::InvalidResponse("Bluesky token response omitted its subject".into())
    })?;
    let session = sessions
        .get(&subject)
        .await
        .map_err(|error| OAuthError::Provider(error.to_string()))?
        .ok_or_else(|| {
            OAuthError::InvalidResponse("Bluesky OAuth session was not retained".into())
        })?;
    let scopes = session
        .token_set
        .scope
        .as_deref()
        .map(|scope| scope.split_ascii_whitespace().collect::<Vec<_>>())
        .unwrap_or_default();
    if !scopes.contains(&"atproto") || !scopes.contains(&"transition:generic") {
        return Err(OAuthError::InvalidResponse(
            "Bluesky token response did not grant atproto and transition:generic".into(),
        ));
    }
    if session.token_set.sub != subject
        || session.token_set.iss.is_empty()
        || session.token_set.aud.is_empty()
    {
        return Err(OAuthError::InvalidResponse(
            "Bluesky issuer, subject, or audience validation failed".into(),
        ));
    }
    Ok(BlueskyOAuthCredential {
        subject,
        session,
        client: stored_client,
    })
}

#[derive(Clone)]
struct HardenedHttpClient;

impl HardenedHttpClient {
    fn new() -> Result<Self, OAuthError> {
        Ok(Self)
    }
}

impl HttpClient for HardenedHttpClient {
    async fn send_http(
        &self,
        request: Request<Vec<u8>>,
    ) -> Result<Response<Vec<u8>>, Box<dyn Error + Send + Sync + 'static>> {
        let request: reqwest::Request = request.try_into()?;
        let client = pinned_client(request.url()).await?;
        let response = client.execute(request).await?;
        if response
            .content_length()
            .is_some_and(|size| size > MAX_HTTP_BODY_BYTES as u64)
        {
            return Err("OAuth response exceeded the size limit".into());
        }
        let mut builder = Response::builder().status(response.status());
        for (name, value) in response.headers() {
            builder = builder.header(name, value);
        }
        let bytes = response.bytes().await?;
        if bytes.len() > MAX_HTTP_BODY_BYTES {
            return Err("OAuth response exceeded the size limit".into());
        }
        Ok(builder.body(bytes.to_vec())?)
    }
}

#[derive(Clone, Copy)]
struct NoDnsResolver;

impl DnsTxtResolver for NoDnsResolver {
    async fn resolve(
        &self,
        _: &str,
    ) -> Result<Vec<String>, Box<dyn Error + Send + Sync + 'static>> {
        Ok(Vec::new())
    }
}

fn provider_request(error: reqwest::Error) -> OAuthError {
    OAuthError::Provider(error.without_url().to_string())
}
