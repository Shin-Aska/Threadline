use super::*;
use crate::providers::SocialProvider;
use async_trait::async_trait;
use base64::{engine::general_purpose::STANDARD, Engine};
use std::sync::{Mutex, RwLock};
struct RecordingProvider(Mutex<Vec<PreparedPost>>);
struct UnusedCredentials;
impl crate::credentials::CredentialStore for UnusedCredentials {
    fn set(&self, _: &str, _: &str) -> Result<(), AppError> {
        Ok(())
    }
    fn get(&self, _: &str) -> Result<String, AppError> {
        Err(AppError::Credential("unused".into()))
    }
    fn delete(&self, _: &str) -> Result<(), AppError> {
        Ok(())
    }
}
#[async_trait]
impl SocialProvider for RecordingProvider {
    async fn capabilities(&self) -> Result<PlatformCapabilities, AppError> {
        Ok(default_capabilities(300, false))
    }
    async fn publish(&self, post: PreparedPost) -> Result<PublishedPost, AppError> {
        let mut posts = self.0.lock().expect("posts");
        posts.push(post);
        Ok(PublishedPost {
            remote_id: posts.len().to_string(),
            remote_cid: None,
            root_id: None,
            root_cid: None,
        })
    }
    async fn reply(
        &self,
        _: &PublishedPost,
        post: PreparedPost,
    ) -> Result<PublishedPost, AppError> {
        self.publish(post).await
    }
}
#[tokio::test]
async fn images_attach_only_to_the_first_thread_post() {
    let mut account = crate::accounts::mock_accounts().remove(0);
    account.id = "test".into();
    account.capabilities.max_text_length = 18;
    let database = crate::database::Database::in_memory().expect("database");
    database
        .seed(std::slice::from_ref(&account))
        .expect("account");
    let provider = Arc::new(RecordingProvider(Mutex::new(Vec::new())));
    let mut providers = crate::config::ProviderMap::new();
    providers.insert(account.id.clone(), provider.clone());
    let state = AppState {
        database,
        providers: RwLock::new(providers),
        credentials: Arc::new(UnusedCredentials),
        oauth: crate::oauth::coordinator::OAuthCoordinator::default(),
    };
    let draft = state
        .database
        .save_draft(SaveDraftInput {
            id: None,
            expected_revision: None,
            post: CanonicalPost {
                text: "one two three four five six seven eight".into(),
                media: vec![MediaAttachment {
                    id: "image".into(),
                    name: "image.png".into(),
                    mime_type: "image/png".into(),
                    size_bytes: 8,
                    alt_text: "Description".into(),
                    data_base64: STANDARD.encode(b"\x89PNG\r\n\x1a\n"),
                    duration_ms: None,
                }],
                policy: PublishingPolicy::Adaptive,
                destination_account_ids: vec![account.id],
            },
        })
        .expect("draft");
    let result = crate::publishing::publish_draft(&state, &draft.id)
        .await
        .expect("publication");
    assert!(matches!(
        result.destinations[0].status,
        PublicationOutcome::Published
    ));
    let posts = provider.0.lock().expect("posts");
    assert_eq!(
        posts
            .iter()
            .map(|post| post.media.len())
            .collect::<Vec<_>>(),
        vec![1, 0, 0]
    );
    assert_eq!(posts[0].media[0].alt_text, "Description");
}
#[test]
fn image_only_drafts_have_one_native_preview_part() {
    let account = crate::accounts::mock_accounts().remove(0);
    let post = CanonicalPost {
        text: String::new(),
        media: vec![MediaAttachment {
            id: "image".into(),
            name: "image.png".into(),
            mime_type: "image/png".into(),
            size_bytes: 100,
            alt_text: "Description".into(),
            data_base64: String::new(),
            duration_ms: None,
        }],
        policy: PublishingPolicy::CommonLimit,
        destination_account_ids: vec![account.id.clone()],
    };
    let preview = crate::composer::preview(&post, &[account]).expect("image preview");
    assert_eq!(preview.destinations[0].parts, vec![String::new()]);
}
