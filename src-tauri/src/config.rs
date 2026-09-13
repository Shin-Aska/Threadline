use crate::{
    models::{Account, CountingPolicy, PlatformCapabilities, ProviderKind},
    providers::{bluesky::BlueskyProvider, mastodon::MastodonProvider, SocialProvider},
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, env, sync::Arc};

pub type ProviderMap = HashMap<String, Arc<dyn SocialProvider>>;

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

pub fn provider_from_credential(
    account: &Account,
    encoded: &str,
) -> Option<Arc<dyn SocialProvider>> {
    let credential: StoredCredential = serde_json::from_str(encoded).ok()?;
    let client = reqwest::Client::builder()
        .user_agent(concat!("Threadline/", env!("CARGO_PKG_VERSION")))
        .build()
        .ok()?;
    match credential {
        StoredCredential::Bluesky {
            service_url,
            identifier,
            app_password,
        } => Some(Arc::new(BlueskyProvider {
            capabilities: account.capabilities.clone(),
            client,
            service_url,
            identifier,
            app_password,
        })),
        StoredCredential::Mastodon {
            base_url,
            access_token,
        } => Some(Arc::new(MastodonProvider {
            capabilities: account.capabilities.clone(),
            client,
            base_url,
            access_token,
        })),
    }
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
        supported_media_types: vec!["image/jpeg".into(), "image/png".into(), "video/mp4".into()],
        supports_polls: mastodon,
        supports_content_warnings: mastodon,
    }
}

/// Loads opt-in live accounts without ever persisting or logging their secrets.
/// Both the prefixed names and common CI secret names are accepted.
pub fn live_accounts() -> (Vec<Account>, ProviderMap) {
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
        providers.insert(
            id.clone(),
            Arc::new(BlueskyProvider {
                capabilities: caps.clone(),
                client: client.clone(),
                service_url: value(&["THREADLINE_BSKY_SERVICE", "BSKY_SERVICE"])
                    .unwrap_or_else(|| "https://bsky.social".into()),
                identifier: handle.clone(),
                app_password,
            }),
        );
        accounts.push(Account {
            id,
            provider: ProviderKind::Bluesky,
            handle: handle.clone(),
            display_name: handle,
            instance_url: None,
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
