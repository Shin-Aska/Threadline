use crate::models::*;
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
pub fn mock_accounts() -> Vec<Account> {
    vec![
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
        Account {
            id: "mastodon-social".into(),
            provider: ProviderKind::Mastodon,
            handle: "@river@mastodon.social".into(),
            display_name: "River".into(),
            instance_url: Some("https://mastodon.social".into()),
            did: None,
            capabilities: caps(500, true, true, 4),
        },
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
