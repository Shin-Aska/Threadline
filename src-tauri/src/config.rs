use crate::{
    credentials::CredentialStore,
    models::{Account, CountingPolicy, PlatformCapabilities, ProviderKind},
    oauth::bluesky::BlueskyOAuthCredential,
    providers::{bluesky::BlueskyProvider, mastodon::MastodonProvider, SocialProvider},
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, env, sync::Arc};

pub type ProviderMap = HashMap<String, Arc<dyn SocialProvider>>;

enum ParsedCredential {
    BlueskyOAuth(Box<BlueskyOAuthCredential>),
    Legacy(StoredCredential),
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "provider", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum StoredCredential {
    Bluesky {
        service_url: String,
        identifier: String,
        app_password: String,
    },
    Mastodon {
        base_url: String,
        access_token: String,
    },
}

pub async fn provider_from_credential(
    account: &Account,
    encoded: &str,
) -> Option<Arc<dyn SocialProvider>> {
    provider_from_credential_with_persistence(account, encoded, None).await
}

pub async fn provider_from_credential_with_persistence(
    account: &Account,
    encoded: &str,
    persistence: Option<Arc<dyn CredentialStore>>,
) -> Option<Arc<dyn SocialProvider>> {
    let credential = parse_credential(encoded)?;
    let client = reqwest::Client::builder()
        .user_agent(concat!("Threadline/", env!("CARGO_PKG_VERSION")))
        .build()
        .ok()?;
    match credential {
        ParsedCredential::BlueskyOAuth(credential) => {
            let runtime = credential
                .restore_with_persistence(persistence)
                .await
                .ok()?;
            Some(Arc::new(BlueskyProvider {
                app_password_session: Default::default(),
                capabilities: account.capabilities.clone(),
                client,
                service_url: runtime.service_url(),
                identifier: runtime.subject().into(),
                app_password: String::new(),
                oauth: Some(Arc::new(runtime)),
            }))
        }
        ParsedCredential::Legacy(StoredCredential::Bluesky {
            service_url,
            identifier,
            app_password,
        }) => Some(Arc::new(BlueskyProvider {
            app_password_session: Default::default(),
            capabilities: account.capabilities.clone(),
            client,
            service_url,
            identifier,
            app_password,
            oauth: None,
        })),
        ParsedCredential::Legacy(StoredCredential::Mastodon {
            base_url,
            access_token,
        }) => Some(Arc::new(MastodonProvider {
            capabilities: account.capabilities.clone(),
            client,
            base_url,
            access_token,
        })),
    }
}

fn parse_credential(encoded: &str) -> Option<ParsedCredential> {
    serde_json::from_str::<BlueskyOAuthCredential>(encoded)
        .map(|credential| ParsedCredential::BlueskyOAuth(Box::new(credential)))
        .or_else(|_| serde_json::from_str(encoded).map(ParsedCredential::Legacy))
        .ok()
}

fn value(names: &[&str]) -> Option<String> {
    names
        .iter()
        .find_map(|name| env::var(name).ok().filter(|value| !value.trim().is_empty()))
}

fn capabilities(max_text_length: usize, mastodon: bool) -> PlatformCapabilities {
    PlatformCapabilities {
        max_text_length,
        counting_policy: CountingPolicy::Grapheme,
        reserved_url_length: mastodon.then_some(23),
        max_media_attachments: 4,
        supported_media_types: vec![
            "image/jpeg".into(),
            "image/png".into(),
            "image/webp".into(),
            "video/mp4".into(),
        ],
        max_video_bytes: (!mastodon).then_some(300_000_000),
        max_video_duration_ms: None,
        supports_polls: mastodon,
        supports_content_warnings: mastodon,
    }
}

/// Loads opt-in live accounts without ever persisting or logging their secrets.
/// Both the prefixed names and common CI secret names are accepted.
pub fn live_accounts() -> (Vec<Account>, ProviderMap) {
    live_accounts_with(value)
}
fn live_accounts_with(value: impl Fn(&[&str]) -> Option<String>) -> (Vec<Account>, ProviderMap) {
    let client = reqwest::Client::builder()
        .user_agent(concat!("Threadline/", env!("CARGO_PKG_VERSION")))
        .build()
        .expect("reqwest client configuration is valid");
    let mut accounts = Vec::new();
    let mut providers: ProviderMap = HashMap::new();

    let bsky_handle = value(&["THREADLINE_BSKY_HANDLE", "BSKY_HANDLE", "BLUESKY_HANDLE"]);
    let bsky_password = value(&[
        "THREADLINE_BSKY_APP_PASSWORD",
        "BSKY_APP_PASSWORD",
        "BLUESKY_APP_PASSWORD",
    ]);
    if let (Some(handle), Some(app_password)) = (bsky_handle, bsky_password) {
        let id = "bsky-env".to_owned();
        let caps = capabilities(300, false);
        let service_url = value(&["THREADLINE_BSKY_SERVICE", "BSKY_SERVICE"])
            .unwrap_or_else(|| "https://bsky.social".into());
        providers.insert(
            id.clone(),
            Arc::new(BlueskyProvider {
                app_password_session: Default::default(),
                capabilities: caps.clone(),
                client: client.clone(),
                service_url: service_url.clone(),
                identifier: handle.clone(),
                app_password,
                oauth: None,
            }),
        );
        accounts.push(Account {
            id,
            provider: ProviderKind::Bluesky,
            handle: handle.clone(),
            display_name: handle,
            instance_url: Some(service_url),
            did: None,
            capabilities: caps,
        });
    }

    let mastodon_url = value(&[
        "THREADLINE_MASTODON_BASE_URL",
        "MASTODON_BASE_URL",
        "MASTODON_URL",
        "MASTODON_INSTANCE",
    ]);
    let mastodon_token = value(&[
        "THREADLINE_MASTODON_ACCESS_TOKEN",
        "MASTODON_ACCESS_TOKEN",
        "MASTODON_TOKEN",
    ]);
    if let (Some(base_url), Some(access_token)) = (mastodon_url, mastodon_token) {
        let id = "mastodon-env".to_owned();
        let handle = value(&["THREADLINE_MASTODON_HANDLE", "MASTODON_HANDLE"])
            .unwrap_or_else(|| base_url.clone());
        let caps = capabilities(
            value(&["THREADLINE_MASTODON_MAX_LENGTH"])
                .and_then(|value| value.parse().ok())
                .unwrap_or(500),
            true,
        );
        providers.insert(
            id.clone(),
            Arc::new(MastodonProvider {
                capabilities: caps.clone(),
                client,
                base_url: base_url.clone(),
                access_token,
            }),
        );
        accounts.push(Account {
            id,
            provider: ProviderKind::Mastodon,
            handle: handle.clone(),
            display_name: handle,
            instance_url: Some(base_url),
            did: None,
            capabilities: caps,
        });
    }
    (accounts, providers)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn oauth_credential_fixture() -> &'static str {
        r#"{
            "subject":"did:plc:restartfixture",
            "session":{
                "dpop_key":{
                    "kty":"EC",
                    "crv":"P-256",
                    "x":"NIRNgPVAwnVNzN5g2Ik2IMghWcjnBOGo9B-lKXSSXFs",
                    "y":"iWF-Of43XoSTZxcadO9KWdPTjiCoviSztYw7aMtZZMc",
                    "d":"9MuCYfKK4hf95p_VRj6cxKJwORTgvEU3vynfmSgFH2M"
                },
                "token_set":{
                    "iss":"https://issuer.example",
                    "sub":"did:plc:restartfixture",
                    "aud":"https://pds.example",
                    "scope":"atproto transition:generic",
                    "refresh_token":"refresh-secret",
                    "access_token":"access-secret",
                    "token_type":"DPoP",
                    "expires_at":null
                }
            },
            "client":{
                "mode":"LOCALHOST",
                "redirect_uri":"http://127.0.0.1:49152/oauth/callback"
            }
        }"#
    }

    #[test]
    fn persisted_oauth_session_is_distinguished_from_legacy_credentials() {
        // Given a keychain-shaped Bluesky OAuth session containing refresh and DPoP material.
        let encoded = oauth_credential_fixture();

        // When startup parses the stored credential format.
        let credential = parse_credential(encoded).expect("OAuth credential");

        // Then it selects the OAuth restore path and retains the verified subject.
        match credential {
            ParsedCredential::BlueskyOAuth(credential) => {
                assert_eq!(credential.did(), "did:plc:restartfixture");
            }
            ParsedCredential::Legacy(_) => panic!("OAuth session parsed as a legacy secret"),
        }
    }

    #[test]
    fn custom_environment_service_is_retained_in_public_account_metadata() {
        // Given explicit environment settings without changing process-global variables.
        let settings = HashMap::from([
            ("THREADLINE_BSKY_HANDLE", "custom.test"),
            ("THREADLINE_BSKY_APP_PASSWORD", "test-password"),
            ("THREADLINE_BSKY_SERVICE", "https://custom.example"),
        ]);
        // When environment account metadata is created.
        let (accounts, _) = live_accounts_with(|names| {
            names
                .iter()
                .find_map(|name| settings.get(name).map(|value| (*value).to_owned()))
        });
        // Then reconnect can use the same custom endpoint.
        assert_eq!(accounts.len(), 1);
        assert_eq!(
            accounts[0].instance_url.as_deref(),
            Some("https://custom.example")
        );
    }
}
