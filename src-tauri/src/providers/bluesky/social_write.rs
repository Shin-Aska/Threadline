use crate::{
    error::AppError,
    models::{ProviderKind, PublishedPost},
    providers::social::{
        Actor, PostMetrics, SocialAction, SocialActionResult, SocialPost, ViewerState,
    },
};

use super::{native, BlueskyProvider, RecordResponse};

impl BlueskyProvider {
    async fn create_social_record(
        &self,
        collection: &str,
        record: serde_json::Value,
    ) -> Result<RecordResponse, AppError> {
        if let Some(oauth) = &self.oauth {
            return self
                .post_json(
                    "com.atproto.repo.createRecord",
                    serde_json::json!({"repo": oauth.subject(), "collection": collection, "record": record}),
                )
                .await;
        }
        let session = self.session().await?;
        self.request(
            self.client
                .post(format!(
                    "{}/xrpc/com.atproto.repo.createRecord",
                    self.service_url.trim_end_matches('/')
                ))
                .bearer_auth(&session.access_jwt),
            serde_json::json!({"repo": session.did, "collection": collection, "record": record}),
        )
        .await
    }

    async fn delete_social_record(&self, uri: &str) -> Result<(), AppError> {
        let mut parts = uri.split('/');
        let scheme = parts.next();
        let empty = parts.next();
        let repo = parts.next();
        let collection = parts.next();
        let rkey = parts.next();
        if scheme != Some("at:") || empty != Some("") || parts.next().is_some() {
            return Err(AppError::Provider("Bluesky record URI is invalid".into()));
        }
        let (repo, collection, rkey) = match (repo, collection, rkey) {
            (Some(repo), Some(collection), Some(rkey)) => (repo, collection, rkey),
            _ => {
                return Err(AppError::Provider(
                    "Bluesky record URI is incomplete".into(),
                ))
            }
        };
        let _: serde_json::Value = self
            .post_json(
                "com.atproto.repo.deleteRecord",
                serde_json::json!({"repo": repo, "collection": collection, "rkey": rkey}),
            )
            .await?;
        Ok(())
    }

    pub(super) async fn resolve_post(&self, post_id: &str) -> Result<native::PostView, AppError> {
        let response: native::PostsResponse = self
            .social_get("app.bsky.feed.getPosts", &[("uris", post_id)])
            .await?;
        response
            .posts
            .into_iter()
            .next()
            .ok_or_else(|| AppError::Provider("Bluesky post is unavailable".into()))
    }

    pub(super) async fn apply_social_action(
        &self,
        action: SocialAction,
    ) -> Result<SocialActionResult, AppError> {
        match action {
            SocialAction::Like { post_id } => {
                self.subject_action(&post_id, "app.bsky.feed.like").await
            }
            SocialAction::Repost { post_id } => {
                self.subject_action(&post_id, "app.bsky.feed.repost").await
            }
            SocialAction::Unlike { post_id } => self.undo_post_action(&post_id, true).await,
            SocialAction::UndoRepost { post_id } => self.undo_post_action(&post_id, false).await,
            SocialAction::Follow { profile_id } => self.follow_action(&profile_id).await,
            SocialAction::Unfollow { profile_id } => self.unfollow_action(&profile_id).await,
            SocialAction::Reply { post_id, text } => self.reply_action(post_id, text).await,
        }
    }

    async fn subject_action(
        &self,
        post_id: &str,
        collection: &str,
    ) -> Result<SocialActionResult, AppError> {
        let post = self.resolve_post(post_id).await?;
        let record = self.create_social_record(collection, serde_json::json!({"$type": collection, "subject": {"uri": post.uri, "cid": post.cid}, "createdAt": crate::providers::now_iso8601()})).await?;
        let liked = collection == "app.bsky.feed.like";
        Ok(SocialActionResult {
            target_id: post_id.to_owned(),
            viewer: Some(ViewerState {
                liked,
                reposted: !liked,
                like_uri: liked.then(|| record.uri.clone()),
                repost_uri: (!liked).then(|| record.uri.clone()),
            }),
            followed: None,
            record_id: Some(record.uri),
            created_post: None,
        })
    }

    async fn undo_post_action(
        &self,
        post_id: &str,
        like: bool,
    ) -> Result<SocialActionResult, AppError> {
        let post = self.resolve_post(post_id).await?;
        let viewer = post
            .viewer
            .ok_or_else(|| AppError::Provider("Bluesky viewer state is unavailable".into()))?;
        let record = if like {
            viewer.like.clone()
        } else {
            viewer.repost.clone()
        }
        .ok_or_else(|| {
            AppError::Provider("Bluesky action is not active for this account".into())
        })?;
        self.delete_social_record(&record).await?;
        Ok(SocialActionResult {
            target_id: post_id.to_owned(),
            viewer: Some(ViewerState {
                liked: !like && viewer.like.is_some(),
                reposted: like && viewer.repost.is_some(),
                like_uri: (!like).then_some(viewer.like).flatten(),
                repost_uri: like.then_some(viewer.repost).flatten(),
            }),
            followed: None,
            record_id: None,
            created_post: None,
        })
    }

    async fn follow_action(&self, profile_id: &str) -> Result<SocialActionResult, AppError> {
        let record = self.create_social_record("app.bsky.graph.follow", serde_json::json!({"$type": "app.bsky.graph.follow", "subject": profile_id, "createdAt": crate::providers::now_iso8601()})).await?;
        Ok(SocialActionResult {
            target_id: profile_id.to_owned(),
            viewer: None,
            followed: Some(true),
            record_id: Some(record.uri),
            created_post: None,
        })
    }

    async fn unfollow_action(&self, profile_id: &str) -> Result<SocialActionResult, AppError> {
        let profile = self.profile_details(profile_id).await?;
        let record = profile.follow_uri.ok_or_else(|| {
            AppError::Provider("Bluesky profile is not followed by this account".into())
        })?;
        self.delete_social_record(&record).await?;
        Ok(SocialActionResult {
            target_id: profile_id.to_owned(),
            viewer: None,
            followed: Some(false),
            record_id: None,
            created_post: None,
        })
    }

    async fn reply_action(
        &self,
        post_id: String,
        text: String,
    ) -> Result<SocialActionResult, AppError> {
        let parent = self.resolve_post(&post_id).await?;
        let root = parent
            .record
            .reply
            .as_ref()
            .map(|reply| reply.root.clone())
            .unwrap_or(native::StrongRef {
                uri: parent.uri.clone(),
                cid: parent.cid.clone(),
            });
        let (did, handle) = match &self.oauth {
            Some(oauth) => (oauth.subject().to_owned(), oauth.subject().to_owned()),
            None => {
                let session = self.session().await?;
                (session.did, session.handle)
            }
        };
        let created_at = crate::providers::now_iso8601();
        let published = self
            .create_record(
                crate::models::PreparedPost {
                    text: text.clone(),
                    media: Vec::new(),
                },
                Some(&PublishedPost {
                    remote_id: parent.uri.clone(),
                    remote_cid: Some(parent.cid),
                    root_id: Some(root.uri.clone()),
                    root_cid: Some(root.cid.clone()),
                }),
            )
            .await?;
        // The App View may not index this PDS write before the action returns.
        let rkey = published.remote_id.rsplit('/').next().unwrap_or_default();
        let created_post = SocialPost {
            canonical_key: format!("BLUESKY:{}", published.remote_id),
            provider: ProviderKind::Bluesky,
            remote_id: published.remote_id.clone(),
            remote_cid: published.remote_cid,
            remote_url: format!("https://bsky.app/profile/{did}/post/{rkey}"),
            author: Actor {
                id: did,
                display_name: handle.clone(),
                handle,
                avatar_url: None,
            },
            text,
            created_at,
            media: Vec::new(),
            metrics: PostMetrics {
                replies: Some(0),
                reposts: Some(0),
                likes: Some(0),
            },
            viewer: ViewerState::default(),
            reply_parent_id: Some(parent.uri),
            reply_root_id: Some(root.uri),
            reply_root_cid: Some(root.cid),
        };
        Ok(SocialActionResult {
            target_id: post_id,
            viewer: None,
            followed: None,
            record_id: Some(published.remote_id),
            created_post: Some(created_post),
        })
    }
}
