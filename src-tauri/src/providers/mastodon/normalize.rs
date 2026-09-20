use crate::{
    error::AppError,
    models::ProviderKind,
    providers::social::{Actor, Media, PostMetrics, SocialPost, ViewerState},
};

use super::native;

pub(super) fn actor(value: native::Account) -> Actor {
    let display_name = if value.display_name.trim().is_empty() {
        value.acct.clone()
    } else {
        value.display_name
    };
    Actor {
        id: value.id,
        display_name,
        handle: value.acct,
        avatar_url: value.avatar,
    }
}

pub(super) fn post(base_url: &str, value: native::Status) -> Result<SocialPost, AppError> {
    let value = value.reblog.as_deref().cloned().unwrap_or(value);
    let cleaner =
        regex::Regex::new("<[^>]+>").map_err(|error| AppError::Provider(error.to_string()))?;
    let text = cleaner.replace_all(&value.content, "").to_string();
    let reply_parent_id = value.in_reply_to_id.clone();
    Ok(SocialPost {
        canonical_key: format!("MASTODON:{base_url}:{}", value.id),
        provider: ProviderKind::Mastodon,
        remote_url: value.url.unwrap_or_default(),
        remote_id: value.id,
        remote_cid: None,
        author: actor(value.account),
        text,
        created_at: value.created_at,
        media: value
            .media_attachments
            .into_iter()
            .map(|media| Media {
                url: media.url,
                alt: media.description.unwrap_or_default(),
                media_type: media.media_type,
            })
            .collect(),
        metrics: PostMetrics {
            replies: value.replies_count,
            reposts: value.reblogs_count,
            likes: value.favourites_count,
        },
        viewer: ViewerState {
            liked: value.favourited,
            reposted: value.reblogged,
            like_uri: None,
            repost_uri: None,
        },
        reply_parent_id,
        reply_root_id: None,
        reply_root_cid: None,
    })
}
