//! Fixed sample accounts used by local demonstrations and selected tests.
//!
//! These records are fixtures, not credentials or live provider connections.

use crate::models::*;

/// Builds the common capability values used by the sample destinations.
///
/// Keeping the shared text-counting and media settings here makes differences
/// between fixtures explicit at their call sites. The arguments capture the
/// limits that vary in the examples; video limits remain unknown because these
/// fixtures do not model provider video constraints.
fn caps(max: usize, polls: bool, cw: bool, media: usize) -> PlatformCapabilities {
    PlatformCapabilities {
        max_text_length: max,
        counting_policy: CountingPolicy::Grapheme,
        reserved_url_length: Some(23),
        max_media_attachments: media,
        supported_media_types: vec!["image/jpeg".into(), "image/png".into(), "video/mp4".into()],
        max_video_bytes: None,
        max_video_duration_ms: None,
        supports_polls: polls,
        supports_content_warnings: cw,
    }
}
/// Returns representative Bluesky and Mastodon accounts with publishing limits.
///
/// The returned order is stable because tests select fixtures by index. The
/// first account omits Mastodon's fixed URL-length reservation. The second is
/// a typical Mastodon destination, and the last gives publishing tests a much
/// larger text and media allowance for comparing destination limits.
pub fn mock_accounts() -> Vec<Account> {
    vec![
        // Bluesky counts URL text at its full grapheme length; Mastodon
        // reserves 23 characters for each URL.
        Account {
            id: "bsky-alice".into(),
            provider: ProviderKind::Bluesky,
            handle: "alice.bsky.social".into(),
            display_name: "Alice".into(),
            instance_url: None,
            did: Some("did:plc:threadline-alice".into()),
            capabilities: PlatformCapabilities {
                reserved_url_length: None,
                ..caps(300, false, false, 4)
            },
        },
        // A standard Mastodon fixture with polls and content warnings enabled.
        Account {
            id: "mastodon-social".into(),
            provider: ProviderKind::Mastodon,
            handle: "@river@mastodon.social".into(),
            display_name: "River".into(),
            instance_url: Some("https://mastodon.social".into()),
            did: None,
            capabilities: caps(500, true, true, 4),
        },
        // A second Mastodon server with deliberately larger limits for tests
        // that compare destinations or need long posts and larger media sets.
        Account {
            id: "mastodon-long".into(),
            provider: ProviderKind::Mastodon,
            handle: "@sora@long.example".into(),
            display_name: "Sora".into(),
            instance_url: Some("https://long.example".into()),
            did: None,
            capabilities: caps(5000, true, true, 8),
        },
    ]
}
