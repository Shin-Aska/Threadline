use super::*;
use crate::{
    accounts::mock_accounts,
    config::ProviderMap,
    credentials::CredentialStore,
    database::Database,
    models::{
        CanonicalPost, CreateScheduleInput, PlatformCapabilities, PublishingPolicy, SaveDraftInput,
    },
};
use async_trait::async_trait;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex, RwLock,
};

struct UnusedCredentials;

impl CredentialStore for UnusedCredentials {
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

struct SecondSegmentUncertain {
    calls: AtomicUsize,
    capabilities: PlatformCapabilities,
}

#[async_trait]
impl SocialProvider for SecondSegmentUncertain {
    async fn capabilities(&self) -> Result<PlatformCapabilities, AppError> {
        Ok(self.capabilities.clone())
    }

    async fn publish(&self, _: PreparedPost) -> Result<PublishedPost, AppError> {
        let call = self.calls.fetch_add(1, Ordering::SeqCst);
        tokio::task::yield_now().await;
        Ok(PublishedPost {
            remote_id: format!("remote-{call}"),
            remote_cid: None,
            root_id: None,
            root_cid: None,
        })
    }

    async fn reply(&self, _: &PublishedPost, _: PreparedPost) -> Result<PublishedPost, AppError> {
        self.calls.fetch_add(1, Ordering::SeqCst);
        Err(AppError::Provider("response was lost".into()))
    }
}

fn state() -> (AppState, Arc<SecondSegmentUncertain>) {
    let mut account = mock_accounts().remove(0);
    account.id = "destination".into();
    account.capabilities.max_text_length = 18;
    let database = Database::in_memory().expect("database");
    database
        .seed(std::slice::from_ref(&account))
        .expect("account");
    let provider = Arc::new(SecondSegmentUncertain {
        calls: AtomicUsize::new(0),
        capabilities: account.capabilities,
    });
    let mut providers = ProviderMap::new();
    providers.insert(account.id, provider.clone());
    (
        AppState {
            database,
            providers: RwLock::new(providers),
            credentials: Arc::new(UnusedCredentials),
            oauth: crate::oauth::coordinator::OAuthCoordinator::default(),
        },
        provider,
    )
}

fn draft(state: &AppState) -> String {
    state
        .database
        .save_draft(SaveDraftInput {
            id: None,
            expected_revision: None,
            post: CanonicalPost {
                text: "This post is long enough to become a thread with several parts.".into(),
                media: Vec::new(),
                policy: PublishingPolicy::Adaptive,
                destination_account_ids: vec!["destination".into()],
            },
        })
        .expect("draft")
        .id
}

#[tokio::test]
async fn partial_thread_records_successful_segment_and_never_retries_uncertain_result() {
    // Given a provider that succeeds once and loses the reply response.
    let (state, provider) = state();
    let draft_id = draft(&state);

    // When the draft is published and the same revision is requested again.
    let first = publish_draft(&state, &draft_id)
        .await
        .expect("first publication");
    let second = publish_draft(&state, &draft_id)
        .await
        .expect("repeat publication");

    // Then the successful prefix is durable and no ambiguous request is replayed.
    assert_eq!(provider.calls.load(Ordering::SeqCst), 2);
    assert_eq!(first.id, second.id);
    assert_eq!(first.destinations[0].status, PublicationOutcome::Uncertain);
    assert_eq!(first.destinations[0].remote_post_ids, ["remote-0"]);
    assert_eq!(
        first.destinations[0].segments[0].status,
        PublicationOutcome::Published
    );
    assert_eq!(
        first.destinations[0].segments[1].status,
        PublicationOutcome::Uncertain
    );
}

#[tokio::test]
async fn concurrent_publish_requests_dispatch_each_destination_once() {
    // Given two callers attempting the same durable draft revision.
    let (state, provider) = state();
    let draft_id = draft(&state);

    // When both calls race through the publication surface.
    let (left, right) = tokio::join!(
        publish_draft(&state, &draft_id),
        publish_draft(&state, &draft_id)
    );

    // Then both return one ledger and the provider sees one thread attempt.
    assert_eq!(left.expect("left").id, right.expect("right").id);
    assert_eq!(provider.calls.load(Ordering::SeqCst), 2);
}

struct RecordingProvider {
    texts: Mutex<Vec<String>>,
    capabilities: PlatformCapabilities,
}

#[async_trait]
impl SocialProvider for RecordingProvider {
    async fn capabilities(&self) -> Result<PlatformCapabilities, AppError> {
        Ok(self.capabilities.clone())
    }

    async fn publish(&self, post: PreparedPost) -> Result<PublishedPost, AppError> {
        let mut texts = self.texts.lock().map_err(|_| AppError::StateUnavailable)?;
        texts.push(post.text);
        Ok(PublishedPost {
            remote_id: format!("remote-{}", texts.len()),
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
async fn scheduled_snapshot_is_distinct_from_later_manually_published_revision() {
    // Given revision A is scheduled before the same draft is edited to revision B.
    let mut account = mock_accounts().remove(0);
    account.id = "destination".into();
    let database = Database::in_memory().expect("database");
    database
        .seed(std::slice::from_ref(&account))
        .expect("account");
    let provider = Arc::new(RecordingProvider {
        texts: Mutex::new(Vec::new()),
        capabilities: account.capabilities,
    });
    let mut providers = ProviderMap::new();
    providers.insert(account.id, provider.clone());
    let state = AppState {
        database,
        providers: RwLock::new(providers),
        credentials: Arc::new(UnusedCredentials),
        oauth: crate::oauth::coordinator::OAuthCoordinator::default(),
    };
    let first = state
        .database
        .save_draft(SaveDraftInput {
            id: None,
            expected_revision: None,
            post: CanonicalPost {
                text: "revision A".into(),
                media: Vec::new(),
                policy: PublishingPolicy::Adaptive,
                destination_account_ids: vec!["destination".into()],
            },
        })
        .expect("first draft");
    let schedule = state
        .database
        .create_schedule(CreateScheduleInput {
            draft_id: first.id.clone(),
            scheduled_for_epoch_ms: 10_000,
            time_zone: "UTC".into(),
        })
        .expect("schedule");
    let second = state
        .database
        .save_draft(SaveDraftInput {
            id: Some(first.id.clone()),
            expected_revision: Some(first.revision),
            post: CanonicalPost {
                text: "revision B".into(),
                media: Vec::new(),
                policy: PublishingPolicy::Adaptive,
                destination_account_ids: vec!["destination".into()],
            },
        })
        .expect("second draft");

    // When revision B is published manually before Send now dispatches the queued item.
    let manual = publish_draft(&state, &second.id)
        .await
        .expect("manual publication");
    let claimed = state
        .database
        .claim_schedule_now(&schedule.id, schedule.revision)
        .expect("schedule claim")
        .expect("claim winner");
    let completed = crate::scheduling::dispatch_schedule(&state, claimed)
        .await
        .expect("scheduled publication");

    // Then the schedule performs a distinct send using immutable revision A.
    let scheduled = completed.result.expect("scheduled result");
    assert_ne!(manual.id, scheduled.id);
    assert_eq!(scheduled.post.text, "revision A");
    assert_eq!(scheduled.draft_revision, first.revision);
    assert_eq!(
        *provider.texts.lock().expect("recorded texts"),
        ["revision B", "revision A"]
    );
}
