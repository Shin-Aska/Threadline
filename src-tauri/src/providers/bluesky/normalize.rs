use crate::{
    models::ProviderKind,
    providers::social::{Actor, Media, PostMetrics, SocialPost, ViewerState},
};

use super::native;

pub(super) fn actor(value: native::Actor) -> Actor {
    Actor {
        id: value.did,
        display_name: value.display_name.unwrap_or_else(|| value.handle.clone()),
        handle: value.handle,
        avatar_url: value.avatar,
    }
}

pub(super) fn post(value: native::PostView) -> SocialPost {
    let handle = value.author.handle.clone();
    let rkey = value.uri.rsplit('/').next().unwrap_or_default();
    let viewer = value.viewer.unwrap_or(native::PostViewer {
        like: None,
        repost: None,
    });
    let reply_parent_id = value
        .record
        .reply
        .as_ref()
        .map(|reply| reply.parent.uri.clone());
    let reply_root_id = value
        .record
        .reply
        .as_ref()
        .map(|reply| reply.root.uri.clone());
    let reply_root_cid = value
        .record
        .reply
        .as_ref()
        .map(|reply| reply.root.cid.clone());
    SocialPost {
        canonical_key: format!("BLUESKY:{}", value.uri),
        provider: ProviderKind::Bluesky,
        remote_url: format!("https://bsky.app/profile/{handle}/post/{rkey}"),
        remote_id: value.uri,
        remote_cid: Some(value.cid),
        author: actor(value.author),
        text: value.record.text,
        created_at: value.record.created_at,
        media: value
            .embed
            .and_then(|embed| embed.images)
            .unwrap_or_default()
            .into_iter()
            .map(|image| Media {
                url: image.fullsize,
                alt: image.alt,
                media_type: "image".into(),
            })
            .collect(),
        metrics: PostMetrics {
            replies: value.reply_count,
            reposts: value.repost_count,
            likes: value.like_count,
        },
        viewer: ViewerState {
            liked: viewer.like.is_some(),
            reposted: viewer.repost.is_some(),
            like_uri: viewer.like,
            repost_uri: viewer.repost,
        },
        reply_parent_id,
        reply_root_id,
        reply_root_cid,
    }
}
