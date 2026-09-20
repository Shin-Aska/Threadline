use super::{
    browser::{BrowserOpener, SystemBrowser},
    callback::LoopbackCallback,
    network::pinned_client,
    OAuthError, DEFAULT_LOGIN_TIMEOUT,
};
use base64::{engine::general_purpose::URL_SAFE_NO_PAD, Engine as _};
use rand::{rngs::OsRng, RngCore};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::time::Duration;
use tokio_util::sync::CancellationToken;
use url::Url;

const CALLBACK_PATH: &str = "/oauth/callback";
const SCOPES: &str = "read:accounts write:statuses write:media";

#[derive(Debug, Clone)]
pub struct MastodonLoginRequest {
    pub instance_url: String,
    pub timeout: Duration,
}

impl MastodonLoginRequest {
    pub fn new(instance_url: String) -> Self {
        Self {
            instance_url,
            timeout: DEFAULT_LOGIN_TIMEOUT,
        }
    }
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MastodonOAuthCredential {
    pub(crate) base_url: String,
    pub(crate) access_token: String,
    pub(crate) remote_id: String,
    pub(crate) handle: String,
    pub(crate) display_name: String,
}

impl MastodonOAuthCredential {
    pub fn account_identity(&self) -> (&str, &str, &str) {
        (&self.remote_id, &self.handle, &self.display_name)
    }

    pub fn into_provider_secret(self) -> (String, String) {
        (self.base_url, self.access_token)
    }
}

#[derive(Deserialize)]
struct AppRegistration {
    client_id: String,
    client_secret: Option<String>,
}

#[derive(Deserialize)]
struct TokenResponse {
    access_token: String,
    scope: Option<String>,
}

#[derive(Deserialize)]
struct VerifiedAccount {
    id: String,
    acct: String,
    display_name: String,
}

pub async fn login(
    request: MastodonLoginRequest,
    cancel: CancellationToken,
) -> Result<MastodonOAuthCredential, OAuthError> {
    login_with_browser(request, cancel, &SystemBrowser).await
}

pub async fn login_with_browser(
    request: MastodonLoginRequest,
    cancel: CancellationToken,
    browser: &dyn BrowserOpener,
) -> Result<MastodonOAuthCredential, OAuthError> {
    let instance = validate_instance(&request.instance_url).await?;
    let client = pinned_client(&instance).await?;
    let callback = LoopbackCallback::bind(CALLBACK_PATH).await?;
    let redirect_uri = callback.redirect_uri()?;
    let registration = register(&client, &instance, &redirect_uri).await?;
    let (verifier, challenge) = pkce();
    let state = nonce();
    let authorize_url = authorization_url(
        &instance,
        &registration.client_id,
        &redirect_uri,
        &challenge,
        &state,
    )?;
    browser.open(authorize_url.as_str())?;
    let callback = callback.wait(Some(&state), request.timeout, cancel).await?;
    let token = exchange(
        &client,
        &instance,
        &registration,
        &redirect_uri,
        &verifier,
        &callback.code,
    )
    .await?;
    validate_scopes(token.scope.as_deref())?;
    verify_account(&client, &instance, token.access_token).await
}

async fn validate_instance(raw: &str) -> Result<Url, OAuthError> {
    let mut url = Url::parse(raw.trim())
        .map_err(|_| OAuthError::Configuration("Mastodon server URL is malformed".into()))?;
    if url.scheme() != "https"
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(OAuthError::Configuration(
            "Mastodon server must be a credential-free HTTPS origin".into(),
        ));
    }
    url.set_path("");
    Ok(url)
}

async fn register(
    client: &reqwest::Client,
    instance: &Url,
    redirect_uri: &str,
) -> Result<AppRegistration, OAuthError> {
    let response = client
        .post(instance.join("api/v1/apps").map_err(invalid_url)?)
        .form(&[
            ("client_name", "Threadline"),
            ("redirect_uris", redirect_uri),
            ("scopes", SCOPES),
        ])
        .send()
        .await
        .map_err(provider_request)?;
    decode_json(response, "Mastodon app registration").await
}

fn authorization_url(
    instance: &Url,
    client_id: &str,
    redirect_uri: &str,
    challenge: &str,
    state: &str,
) -> Result<Url, OAuthError> {
    let mut url = instance.join("oauth/authorize").map_err(invalid_url)?;
    url.query_pairs_mut()
        .append_pair("response_type", "code")
        .append_pair("client_id", client_id)
        .append_pair("redirect_uri", redirect_uri)
        .append_pair("scope", SCOPES)
        .append_pair("code_challenge", challenge)
        .append_pair("code_challenge_method", "S256")
        .append_pair("state", state);
    Ok(url)
}

async fn exchange(
    client: &reqwest::Client,
    instance: &Url,
    registration: &AppRegistration,
    redirect_uri: &str,
    verifier: &str,
    code: &str,
) -> Result<TokenResponse, OAuthError> {
    let mut form = vec![
        ("grant_type", "authorization_code"),
        ("client_id", registration.client_id.as_str()),
        ("redirect_uri", redirect_uri),
        ("code_verifier", verifier),
        ("code", code),
        ("scope", SCOPES),
    ];
    if let Some(secret) = registration
        .client_secret
        .as_deref()
        .filter(|value| !value.is_empty())
    {
        form.push(("client_secret", secret));
    }
    let response = client
        .post(instance.join("oauth/token").map_err(invalid_url)?)
        .form(&form)
        .send()
        .await
        .map_err(provider_request)?;
    decode_json(response, "Mastodon token exchange").await
}

async fn verify_account(
    client: &reqwest::Client,
    instance: &Url,
    access_token: String,
) -> Result<MastodonOAuthCredential, OAuthError> {
    if access_token.is_empty() {
        return Err(OAuthError::InvalidResponse(
            "Mastodon returned an empty access token".into(),
        ));
    }
    let response = client
        .get(
            instance
                .join("api/v1/accounts/verify_credentials")
                .map_err(invalid_url)?,
        )
        .bearer_auth(&access_token)
        .send()
        .await
        .map_err(provider_request)?;
    let account: VerifiedAccount = decode_json(response, "Mastodon credential validation").await?;
    if account.id.is_empty() || account.acct.is_empty() {
        return Err(OAuthError::InvalidResponse(
            "Mastodon account identity is incomplete".into(),
        ));
    }
    Ok(MastodonOAuthCredential {
        base_url: instance.as_str().trim_end_matches('/').into(),
        access_token,
        remote_id: account.id,
        handle: account.acct,
        display_name: account.display_name,
    })
}

async fn decode_json<T: for<'de> Deserialize<'de>>(
    response: reqwest::Response,
    operation: &str,
) -> Result<T, OAuthError> {
    let status = response.status();
    if !status.is_success() {
        return Err(OAuthError::Provider(format!(
            "{operation} returned {status}"
        )));
    }
    response
        .json()
        .await
        .map_err(|_| OAuthError::InvalidResponse(format!("{operation} returned invalid JSON")))
}

fn validate_scopes(scope: Option<&str>) -> Result<(), OAuthError> {
    if let Some(scope) = scope {
        let granted = scope
            .split_ascii_whitespace()
            .collect::<std::collections::HashSet<_>>();
        if !SCOPES
            .split_ascii_whitespace()
            .all(|required| granted.contains(required))
        {
            return Err(OAuthError::InvalidResponse(
                "Mastodon did not grant all requested scopes".into(),
            ));
        }
    }
    Ok(())
}

fn pkce() -> (String, String) {
    let verifier = nonce();
    let challenge = URL_SAFE_NO_PAD.encode(Sha256::digest(verifier.as_bytes()));
    (verifier, challenge)
}

fn nonce() -> String {
    let mut bytes = [0_u8; 32];
    OsRng.fill_bytes(&mut bytes);
    URL_SAFE_NO_PAD.encode(bytes)
}

fn invalid_url(_: url::ParseError) -> OAuthError {
    OAuthError::Configuration("Mastodon endpoint URL is invalid".into())
}

fn provider_request(error: reqwest::Error) -> OAuthError {
    OAuthError::Provider(error.without_url().to_string())
}
