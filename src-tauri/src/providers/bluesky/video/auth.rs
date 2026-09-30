//! Obtains method-scoped video service tokens from app-password or OAuth credentials.

use super::VIDEO_SERVICE_DID;
use crate::{
    error::AppError, oauth::bluesky::BlueskyOAuthRuntime, providers::bluesky::BlueskyProvider,
};
use serde::Deserialize;

/// Short-lived token returned by the PDS service authorization endpoint.
#[derive(Deserialize)]
struct ServiceAuth {
    token: String,
}

/// Credential route used to request video service authorization.
pub(super) enum ServiceAuthSource<'a> {
    Bearer(&'a BlueskyProvider),
    OAuth(&'a BlueskyOAuthRuntime),
}

impl ServiceAuthSource<'_> {
    /// Requests a video service token scoped to the given XRPC method.
    pub(super) async fn token(&self, lxm: &str) -> Result<String, AppError> {
        let auth: ServiceAuth = match self {
            Self::OAuth(oauth) => oauth
                .get_json(
                    "com.atproto.server.getServiceAuth",
                    &[("aud", VIDEO_SERVICE_DID), ("lxm", lxm)],
                )
                .await
                .map_err(|error| AppError::Provider(error.to_string()))?,
            Self::Bearer(provider) => {
                provider
                    .get_json(
                        "com.atproto.server.getServiceAuth",
                        &[("aud", VIDEO_SERVICE_DID), ("lxm", lxm)],
                    )
                    .await?
            }
        };
        Ok(auth.token)
    }
}
