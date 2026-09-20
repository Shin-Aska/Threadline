use super::Database;
use crate::models::{
    CanonicalPost, CreateScheduleInput, MediaAttachment, PublicationOutcome, PublishingPolicy,
    RescheduleInput, SaveDraftInput, ScheduleStatus,
};
use base64::{engine::general_purpose::STANDARD, Engine};
use std::sync::Arc;

fn post(text: &str) -> CanonicalPost {
    CanonicalPost {
        text: text.into(),
        media: vec![MediaAttachment {
            id: "diagram".into(),
            name: "diagram.png".into(),
            mime_type: "image/png".into(),
            size_bytes: 8,
            alt_text: "Diagram".into(),
            data_base64: STANDARD.encode(b"\x89PNG\r\n\x1a\n"),
            duration_ms: None,
        }],
        policy: PublishingPolicy::Adaptive,
        destination_account_ids: vec!["account-one".into()],
    }
}

#[test]
fn draft_and_media_survive_database_reopen() {
    // Given an on-disk database and a draft containing media.
    let root = std::env::temp_dir().join(format!("threadline-reopen-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).expect("temporary root");
    let path = root.join("threadline.sqlite");
    let draft = {
        let db = Database::open(&path).expect("database");
        db.save_draft(SaveDraftInput {
            id: None,
            expected_revision: None,
            post: post("persist me"),
        })
        .expect("save draft")
    };

    // When the database is reopened.
    let reopened = Database::open(&path).expect("reopen database");
    let restored = reopened.get_draft(&draft.id).expect("restore draft");

    // Then its content and exact media bytes are restored.
    assert_eq!(restored.post, post("persist me"));
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn stale_autosave_cannot_overwrite_a_newer_revision() {
    // Given two editor snapshots of the same revision.
    let db = Database::in_memory().expect("database");
    let original = db
        .save_draft(SaveDraftInput {
            id: None,
            expected_revision: None,
            post: post("original"),
        })
        .expect("original draft");
    let saved = db
        .save_draft(SaveDraftInput {
            id: Some(original.id.clone()),
            expected_revision: Some(original.revision),
            post: post("newest"),
        })
        .expect("newer autosave");

    // When the stale snapshot arrives after the newer save.
    let stale = db.save_draft(SaveDraftInput {
        id: Some(original.id.clone()),
        expected_revision: Some(original.revision),
        post: post("stale"),
    });

    // Then it is rejected and the newer content remains durable.
    assert!(stale.is_err());
    assert_eq!(
        db.get_draft(&original.id).expect("draft").post.text,
        "newest"
    );
    assert_eq!(saved.revision, original.revision + 1);
}

#[test]
fn publication_claim_is_single_winner_and_in_flight_recovers_uncertain() {
    // Given two workers sharing one durable draft revision.
    let db = Arc::new(Database::in_memory().expect("database"));
    let draft = db
        .save_draft(SaveDraftInput {
            id: None,
            expected_revision: None,
            post: post("publish once"),
        })
        .expect("draft");

    // When both workers try to create the publication ledger.
    let left = Arc::clone(&db);
    let left_id = draft.id.clone();
    let right = Arc::clone(&db);
    let right_id = draft.id.clone();
    let first = std::thread::spawn(move || left.begin_publication(&left_id));
    let second = std::thread::spawn(move || right.begin_publication(&right_id));
    let first = first.join().expect("first thread").expect("first claim");
    let second = second.join().expect("second thread").expect("second claim");

    // Then both observe the same ledger, and crash recovery protects the in-flight send.
    assert_eq!(first.id, second.id);
    assert!(db
        .claim_destination(&first.id, "account-one")
        .expect("claim"));
    db.recover_interrupted_publications()
        .expect("recover publication");
    let recovered = db.get_publication(&first.id).expect("publication");
    assert_eq!(
        recovered.destinations[0].status,
        PublicationOutcome::Uncertain
    );
    assert!(!db
        .claim_destination(&first.id, "account-one")
        .expect("second claim"));
}

#[test]
fn interrupted_thread_reopens_with_exact_partial_result_and_uncertain_tail() {
    // Given an on-disk ledger with one confirmed segment and the next request in flight.
    let root = std::env::temp_dir().join(format!("threadline-ledger-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).expect("temporary root");
    let path = root.join("threadline.sqlite");
    let publication_id = {
        let db = Database::open(&path).expect("database");
        let draft = db
            .save_draft(SaveDraftInput {
                id: None,
                expected_revision: None,
                post: post("partial thread"),
            })
            .expect("draft");
        let publication = db.begin_publication(&draft.id).expect("publication");
        assert!(db
            .claim_destination(&publication.id, "account-one")
            .expect("claim"));
        assert!(db
            .begin_segment(&publication.id, "account-one", 0)
            .expect("segment zero"));
        db.finish_segment(
            &publication.id,
            "account-one",
            0,
            PublicationOutcome::Published,
            Some("remote-confirmed"),
            None,
        )
        .expect("confirmed segment");
        assert!(db
            .begin_segment(&publication.id, "account-one", 1)
            .expect("segment one"));
        publication.id
    };

    // When Threadline reopens and performs interrupted-send recovery.
    let reopened = Database::open(&path).expect("reopen database");
    reopened
        .recover_interrupted_publications()
        .expect("recover publication");
    let recovered = reopened
        .get_publication(&publication_id)
        .expect("publication");

    // Then the known remote ID remains exact and only the unresolved tail is uncertain.
    assert_eq!(
        recovered.destinations[0].remote_post_ids,
        ["remote-confirmed"]
    );
    assert_eq!(
        recovered.destinations[0].segments[0].status,
        PublicationOutcome::Published
    );
    assert_eq!(
        recovered.destinations[0].segments[1].status,
        PublicationOutcome::Uncertain
    );
    assert_eq!(
        recovered.destinations[0].status,
        PublicationOutcome::Uncertain
    );
    drop(reopened);
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn startup_marks_overdue_schedule_for_review() {
    // Given a queued item whose intended UTC instant elapsed while the app was closed.
    let db = Database::in_memory().expect("database");
    let draft = db
        .save_draft(SaveDraftInput {
            id: None,
            expected_revision: None,
            post: post("scheduled"),
        })
        .expect("draft");
    let schedule = db
        .create_schedule(CreateScheduleInput {
            draft_id: draft.id,
            scheduled_for_epoch_ms: 1_000,
            time_zone: "Asia/Manila".into(),
        })
        .expect("schedule");

    // When startup reviews missed times.
    db.mark_startup_missed(2_000).expect("review missed");

    // Then the item requires explicit attention and cannot be auto-claimed.
    let reviewed = db.get_schedule(&schedule.id).expect("reviewed schedule");
    assert_eq!(reviewed.status, ScheduleStatus::NeedsAttention);
    assert!(db.claim_due_schedule(2_000).expect("claim due").is_none());
}

#[test]
fn wake_after_sleep_flags_stale_schedule_and_claims_only_within_grace() {
    // Given one schedule that will be stale after sleep and another within the one-minute grace.
    let db = Database::in_memory().expect("database");
    let draft = db
        .save_draft(SaveDraftInput {
            id: None,
            expected_revision: None,
            post: post("sleep recovery"),
        })
        .expect("draft");
    let stale = db
        .create_schedule(CreateScheduleInput {
            draft_id: draft.id.clone(),
            scheduled_for_epoch_ms: 100_000,
            time_zone: "UTC".into(),
        })
        .expect("stale schedule");
    let eligible = db
        .create_schedule(CreateScheduleInput {
            draft_id: draft.id,
            scheduled_for_epoch_ms: 190_000,
            time_zone: "UTC".into(),
        })
        .expect("eligible schedule");
    assert!(db
        .claim_due_schedule(90_000)
        .expect("pre-sleep poll")
        .is_none());

    // When the process resumes after its clock advances beyond the first schedule's grace window.
    let claimed = db
        .claim_due_schedule(200_000)
        .expect("post-sleep poll")
        .expect("eligible claim");

    // Then the stale item requires review and the queue continues to the eligible item.
    let stale = db.get_schedule(&stale.id).expect("stale result");
    assert_eq!(stale.status, ScheduleStatus::NeedsAttention);
    assert_eq!(
        stale.attention_reason.as_deref(),
        Some("Scheduled time was missed by more than 1 minute while Threadline was running")
    );
    assert_eq!(claimed.id, eligible.id);
    assert_eq!(claimed.status, ScheduleStatus::Dispatching);
    assert!(db
        .claim_schedule_now(&stale.id, stale.revision)
        .expect("explicit stale send")
        .is_some());
}

#[test]
fn schedule_reopens_with_utc_instant_and_zone_then_can_be_rescheduled() {
    // Given a future schedule stored on disk with its selected IANA zone.
    let root = std::env::temp_dir().join(format!("threadline-schedule-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).expect("temporary root");
    let path = root.join("threadline.sqlite");
    let schedule_id = {
        let db = Database::open(&path).expect("database");
        let draft = db
            .save_draft(SaveDraftInput {
                id: None,
                expected_revision: None,
                post: post("scheduled after restart"),
            })
            .expect("draft");
        db.create_schedule(CreateScheduleInput {
            draft_id: draft.id,
            scheduled_for_epoch_ms: 5_000,
            time_zone: "Asia/Manila".into(),
        })
        .expect("schedule")
        .id
    };

    // When the queue is reopened and the user changes the intended instant and zone.
    let reopened = Database::open(&path).expect("reopen database");
    let original = reopened
        .get_schedule(&schedule_id)
        .expect("original schedule");
    let changed = reopened
        .reschedule(RescheduleInput {
            id: schedule_id,
            expected_revision: original.revision,
            scheduled_for_epoch_ms: 9_000,
            time_zone: "Pacific/Auckland".into(),
        })
        .expect("reschedule");

    // Then the exact instant and display zone are durable and revisioned.
    assert_eq!(changed.scheduled_for_epoch_ms, 9_000);
    assert_eq!(changed.time_zone, "Pacific/Auckland");
    assert_eq!(changed.revision, original.revision + 1);
    drop(reopened);
    std::fs::remove_dir_all(root).expect("cleanup");
}

#[test]
fn send_now_claim_has_one_winner_and_cancelled_item_cannot_dispatch() {
    // Given two queued schedules for the same durable draft.
    let db = Database::in_memory().expect("database");
    let draft = db
        .save_draft(SaveDraftInput {
            id: None,
            expected_revision: None,
            post: post("queue controls"),
        })
        .expect("draft");
    let send = db
        .create_schedule(CreateScheduleInput {
            draft_id: draft.id.clone(),
            scheduled_for_epoch_ms: 10_000,
            time_zone: "UTC".into(),
        })
        .expect("send schedule");
    let cancel = db
        .create_schedule(CreateScheduleInput {
            draft_id: draft.id,
            scheduled_for_epoch_ms: 10_000,
            time_zone: "UTC".into(),
        })
        .expect("cancel schedule");

    // When Send now is claimed twice and the other item is cancelled.
    let first_claim = db
        .claim_schedule_now(&send.id, send.revision)
        .expect("first claim");
    let duplicate_claim = db
        .claim_schedule_now(&send.id, send.revision)
        .expect("duplicate claim");
    let cancelled = db
        .cancel_schedule(&cancel.id, cancel.revision)
        .expect("cancel");

    // Then only one dispatch wins and cancellation is terminal.
    assert!(first_claim.is_some());
    assert!(duplicate_claim.is_none());
    assert_eq!(cancelled.status, ScheduleStatus::Cancelled);
    assert!(db
        .claim_schedule_now(&cancelled.id, cancelled.revision)
        .expect("cancelled claim")
        .is_none());
}

#[test]
fn schedule_rejects_non_iana_zone() {
    // Given a durable draft and an arbitrary display label instead of an IANA zone.
    let db = Database::in_memory().expect("database");
    let draft = db
        .save_draft(SaveDraftInput {
            id: None,
            expected_revision: None,
            post: post("invalid zone"),
        })
        .expect("draft");

    // When the label crosses the scheduling boundary.
    let result = db.create_schedule(CreateScheduleInput {
        draft_id: draft.id,
        scheduled_for_epoch_ms: 10_000,
        time_zone: "not/a-zone".into(),
    });

    // Then it is rejected rather than persisted as an IANA identifier.
    assert!(result.is_err());
}

#[test]
fn deleting_draft_preserves_queued_snapshot() {
    // Given a schedule that captured a draft payload.
    let db = Database::in_memory().expect("database");
    let draft = db
        .save_draft(SaveDraftInput {
            id: None,
            expected_revision: None,
            post: post("retain queued content"),
        })
        .expect("draft");
    let schedule = db
        .create_schedule(CreateScheduleInput {
            draft_id: draft.id.clone(),
            scheduled_for_epoch_ms: 10_000,
            time_zone: "UTC".into(),
        })
        .expect("schedule");

    // When the source draft is explicitly deleted.
    db.delete_draft(&draft.id).expect("delete draft");

    // Then the independent queued payload remains available for review or cancellation.
    let retained = db.get_schedule(&schedule.id).expect("retained schedule");
    assert_eq!(retained.post.text, "retain queued content");
    assert_eq!(retained.status, ScheduleStatus::Queued);
}

#[test]
fn manual_and_scheduled_ledgers_are_distinct_for_same_draft_revision() {
    // Given revision A is both queued and selected for an immediate manual publication.
    let db = Database::in_memory().expect("database");
    let draft = db
        .save_draft(SaveDraftInput {
            id: None,
            expected_revision: None,
            post: post("same revision"),
        })
        .expect("draft");
    let schedule = db
        .create_schedule(CreateScheduleInput {
            draft_id: draft.id.clone(),
            scheduled_for_epoch_ms: 10_000,
            time_zone: "UTC".into(),
        })
        .expect("schedule");

    // When both idempotency scopes create their durable ledgers.
    let manual = db.begin_publication(&draft.id).expect("manual ledger");
    let scheduled = db
        .begin_scheduled_publication(&schedule)
        .expect("scheduled ledger");

    // Then neither suppresses or aliases the other publication effect.
    assert_ne!(manual.id, scheduled.id);
    assert_eq!(manual.draft_revision, scheduled.draft_revision);
    assert_eq!(scheduled.post.text, "same revision");
}

#[test]
fn old_batch_uniqueness_migration_preserves_children_and_schedule_result() {
    // Given a prior on-disk schema with the obsolete draft/revision uniqueness constraint.
    let root = std::env::temp_dir().join(format!("threadline-migration-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&root).expect("temporary root");
    let path = root.join("threadline.sqlite");
    let (publication_id, schedule_id) = {
        let db = Database::open(&path).expect("database");
        let draft = db
            .save_draft(SaveDraftInput {
                id: None,
                expected_revision: None,
                post: post("migrated publication"),
            })
            .expect("draft");
        let schedule = db
            .create_schedule(CreateScheduleInput {
                draft_id: draft.id.clone(),
                scheduled_for_epoch_ms: 10_000,
                time_zone: "UTC".into(),
            })
            .expect("schedule");
        let publication = db.begin_publication(&draft.id).expect("publication");
        assert!(db
            .claim_destination(&publication.id, "account-one")
            .expect("claim"));
        assert!(db
            .begin_segment(&publication.id, "account-one", 0)
            .expect("segment"));
        db.finish_segment(
            &publication.id,
            "account-one",
            0,
            PublicationOutcome::Published,
            Some("remote-migrated"),
            None,
        )
        .expect("segment result");
        db.finish_destination(
            &publication.id,
            &crate::models::DestinationPublication {
                account_id: "account-one".into(),
                status: PublicationOutcome::Published,
                remote_post_ids: vec!["remote-migrated".into()],
                error: None,
                segments: Vec::new(),
            },
        )
        .expect("destination result");
        let claimed = db
            .claim_schedule_now(&schedule.id, schedule.revision)
            .expect("schedule claim")
            .expect("claim winner");
        db.complete_schedule(
            &claimed.id,
            &db.get_publication(&publication.id).expect("result"),
        )
        .expect("complete schedule");
        (publication.id, schedule.id)
    };
    let connection = rusqlite::Connection::open(&path).expect("raw database");
    connection
        .execute_batch(
            "PRAGMA foreign_keys=OFF;
             CREATE TABLE publication_batches_old (
                id TEXT PRIMARY KEY,
                idempotency_key TEXT NOT NULL UNIQUE,
                draft_id TEXT NOT NULL,
                draft_revision INTEGER NOT NULL,
                post_json TEXT NOT NULL,
                created_at_ms INTEGER NOT NULL,
                completed_at_ms INTEGER,
                UNIQUE(draft_id,draft_revision)
             );
             INSERT INTO publication_batches_old SELECT * FROM publication_batches;
             DROP TABLE publication_batches;
             ALTER TABLE publication_batches_old RENAME TO publication_batches;",
        )
        .expect("old schema fixture");
    drop(connection);

    // When the current database opens and rebuilds that parent table.
    let migrated = Database::open(&path).expect("migrated database");
    let publication = migrated
        .get_publication(&publication_id)
        .expect("preserved publication");
    let schedule = migrated
        .get_schedule(&schedule_id)
        .expect("preserved schedule");

    // Then parent, child, segment, schedule result, and foreign keys remain valid.
    assert_eq!(
        publication.destinations[0].remote_post_ids,
        ["remote-migrated"]
    );
    assert_eq!(
        publication.destinations[0].segments[0]
            .remote_post_id
            .as_deref(),
        Some("remote-migrated")
    );
    assert_eq!(schedule.result.expect("schedule result").id, publication_id);
    let foreign_key_errors: i64 = migrated
        .connection()
        .expect("connection")
        .query_row("SELECT COUNT(*) FROM pragma_foreign_key_check", [], |row| {
            row.get(0)
        })
        .expect("foreign key check");
    assert_eq!(foreign_key_errors, 0);
    drop(migrated);
    std::fs::remove_dir_all(root).expect("cleanup");
}
