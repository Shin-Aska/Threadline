use super::*;
use async_trait::async_trait;
use std::sync::Mutex;
struct RecordingProvider(Mutex<Vec<PreparedPost>>);
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
    let provider = RecordingProvider(Mutex::new(Vec::new()));
    let images = [PreparedMedia {
        mime_type: "image/png".into(),
        data: Arc::from(b"image".as_slice()),
        alt_text: "Description".into(),
    }];
    let result = publish_destination(
        "test".into(),
        &["one".into(), "two".into(), "three".into()],
        &images,
        &provider,
    )
    .await;
    assert!(matches!(result.status, PublicationStatus::Published));
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
            mime_type: "image/png".into(),
            size_bytes: 100,
            alt_text: "Description".into(),
            data_base64: String::new(),
        }],
        policy: PublishingPolicy::CommonLimit,
        destination_account_ids: vec![account.id.clone()],
    };
    let preview = crate::composer::preview(&post, &[account]).expect("image preview");
    assert_eq!(preview.destinations[0].parts, vec![String::new()]);
}
