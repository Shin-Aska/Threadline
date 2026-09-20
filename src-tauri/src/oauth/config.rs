use super::OAuthError;
use atrium_oauth::{
    AtprotoClientMetadata, AtprotoLocalhostClientMetadata, AuthMethod, GrantType, KnownScope, Scope,
};
use serde::{Deserialize, Serialize};
use url::Url;

#[derive(Debug, Clone)]
pub enum BlueskyClientMode {
    Localhost(AtprotoLocalhostClientMetadata),
    Hosted(AtprotoClientMetadata),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HostedClientDocument {
    pub client_id: String,
    pub application_type: String,
    pub grant_types: Vec<String>,
    pub scope: String,
    pub response_types: Vec<String>,
    pub redirect_uris: Vec<String>,
    pub dpop_bound_access_tokens: bool,
    pub token_endpoint_auth_method: Option<String>,
}

impl BlueskyClientMode {
    pub fn localhost(redirect_uri: String) -> Self {
        Self::Localhost(AtprotoLocalhostClientMetadata {
            redirect_uris: Some(vec![redirect_uri]),
            scopes: Some(default_scopes()),
        })
    }

    pub fn from_hosted_document(
        metadata_url: &str,
        document: HostedClientDocument,
    ) -> Result<Self, OAuthError> {
        let client_id = validate_hosted_metadata_url(metadata_url)?;
        if document.client_id != metadata_url {
            return Err(OAuthError::InvalidMetadata(
                "client_id must exactly match its metadata URL".into(),
            ));
        }
        if document.application_type != "native"
            || !document.dpop_bound_access_tokens
            || document.token_endpoint_auth_method.as_deref() != Some("none")
        {
            return Err(OAuthError::InvalidMetadata(
                "native public DPoP client metadata is required".into(),
            ));
        }
        if !document
            .grant_types
            .iter()
            .any(|value| value == "authorization_code")
            || !document
                .grant_types
                .iter()
                .any(|value| value == "refresh_token")
            || !document.response_types.iter().any(|value| value == "code")
            || !document
                .scope
                .split_ascii_whitespace()
                .any(|value| value == "atproto")
            || !document
                .scope
                .split_ascii_whitespace()
                .any(|value| value == "transition:generic")
        {
            return Err(OAuthError::InvalidMetadata(
                "authorization code, refresh token, code response, atproto, and transition:generic are required"
                    .into(),
            ));
        }
        let redirect_uri = document.redirect_uris.first().ok_or_else(|| {
            OAuthError::InvalidMetadata("a native redirect URI is required".into())
        })?;
        validate_native_redirect(&client_id, redirect_uri)?;
        Ok(Self::Hosted(AtprotoClientMetadata {
            client_id: document.client_id,
            client_uri: None,
            redirect_uris: document.redirect_uris,
            token_endpoint_auth_method: AuthMethod::None,
            grant_types: vec![GrantType::AuthorizationCode, GrantType::RefreshToken],
            scopes: document
                .scope
                .split_ascii_whitespace()
                .map(|value| match value {
                    "atproto" => Scope::Known(KnownScope::Atproto),
                    "transition:generic" => Scope::Known(KnownScope::TransitionGeneric),
                    "transition:chat.bsky" => Scope::Known(KnownScope::TransitionChatBsky),
                    other => Scope::Unknown(other.into()),
                })
                .collect(),
            jwks_uri: None,
            token_endpoint_auth_signing_alg: None,
        }))
    }
}

fn default_scopes() -> Vec<Scope> {
    vec![
        Scope::Known(KnownScope::Atproto),
        Scope::Known(KnownScope::TransitionGeneric),
    ]
}

pub(crate) fn validate_hosted_metadata_url(raw: &str) -> Result<Url, OAuthError> {
    let url = Url::parse(raw)
        .map_err(|_| OAuthError::InvalidMetadata("client metadata URL is malformed".into()))?;
    if url.scheme() != "https"
        || url.host_str().is_none()
        || url.port().is_some()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
    {
        return Err(OAuthError::InvalidMetadata(
            "client metadata must use a credential-free HTTPS URL without a port or fragment"
                .into(),
        ));
    }
    Ok(url)
}

fn validate_native_redirect(client_id: &Url, raw: &str) -> Result<(), OAuthError> {
    let redirect = Url::parse(raw)
        .map_err(|_| OAuthError::InvalidMetadata("native redirect URI is malformed".into()))?;
    if redirect.scheme() == "https" {
        let same_origin = redirect.host_str() == client_id.host_str()
            && redirect.port_or_known_default() == client_id.port_or_known_default();
        return same_origin.then_some(()).ok_or_else(|| {
            OAuthError::InvalidMetadata(
                "HTTPS native redirect must share the client_id origin".into(),
            )
        });
    }
    let host = client_id
        .host_str()
        .ok_or_else(|| OAuthError::InvalidMetadata("client metadata host is missing".into()))?;
    let expected_scheme = host.split('.').rev().collect::<Vec<_>>().join(".");
    if redirect.scheme() != expected_scheme || raw.contains("://") {
        return Err(OAuthError::InvalidMetadata(
            "custom redirect scheme must be the reversed client_id hostname followed by :/".into(),
        ));
    }
    Ok(())
}
