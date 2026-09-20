use super::{response_json, VIDEO_SERVICE_DID};
use crate::{error::AppError, oauth::bluesky::BlueskyOAuthRuntime};
use serde::Deserialize;
use std::time::Duration;

#[derive(Deserialize)]
struct ServiceAuth {
    token: String,
}

pub(super) enum ServiceAuthSource<'a> {
    Bearer {
        pds_url: &'a str,
        access_jwt: &'a str,
    },
    OAuth(&'a BlueskyOAuthRuntime),
}

impl ServiceAuthSource<'_> {
    pub(super) async fn token(
        &self,
        client: &reqwest::Client,
        lxm: &str,
    ) -> Result<String, AppError> {
        let auth: ServiceAuth = match self {
            Self::OAuth(oauth) => oauth
                .get_json(
                    "com.atproto.server.getServiceAuth",
                    &[("aud", VIDEO_SERVICE_DID), ("lxm", lxm)],
                )
                .await
                .map_err(|error| AppError::Provider(error.to_string()))?,
            Self::Bearer {
                pds_url,
                access_jwt,
            } => {
                response_json(
                    client
                        .get(format!(
                            "{}/xrpc/com.atproto.server.getServiceAuth",
                            pds_url.trim_end_matches('/')
                        ))
                        .query(&[("aud", VIDEO_SERVICE_DID), ("lxm", lxm)])
                        .bearer_auth(access_jwt)
                        .timeout(Duration::from_secs(30)),
                    "Bluesky video service authorization",
                )
                .await?
            }
        };
        Ok(auth.token)
    }
}
