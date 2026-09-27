//! Serializable contracts for durable drafting, publishing, and scheduling.

use super::CanonicalPost;
use serde::{Deserialize, Serialize};

/// Request to create a draft or replace one at a known revision.
///
/// Omit `id` to create a new draft. Updating an existing `id` requires its
/// current `expected_revision`, preventing a stale editor from overwriting it.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SaveDraftInput {
    pub id: Option<String>,
    pub expected_revision: Option<u64>,
    pub post: CanonicalPost,
}

/// Persisted draft with an optimistic-concurrency revision.
///
/// Loaded records include their media bytes in `post`; timestamps are Unix
/// milliseconds for creation and the most recent update.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DraftRecord {
    pub id: String,
    pub revision: u64,
    pub post: CanonicalPost,
    pub created_at_epoch_ms: i64,
    pub updated_at_epoch_ms: i64,
}

/// Durable state of a destination or individual publication segment.
///
/// `InFlight` means a provider call has been claimed but its outcome is not
/// yet durable. If that result is lost, recovery records `Uncertain` rather
/// than retrying a request that might already have published remotely.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum PublicationOutcome {
    /// Durable work has not yet been claimed for dispatch.
    Pending,
    /// A dispatcher owns the provider request, but no terminal result is durable yet.
    InFlight,
    /// The provider confirmed the post and its remote ID is recorded.
    Published,
    /// Dispatch failed before crossing the provider boundary or before an attempt began.
    Failed,
    /// The provider result was lost or ambiguous; automatic retries would risk a duplicate post.
    Uncertain,
    /// The account is unavailable and must be reconnected before publishing.
    Blocked,
}

/// Durable publication state for one destination account.
///
/// `segments` records each text part in a thread. Remote IDs are retained
/// only for posts the provider confirmed, and `error` describes a failure or
/// uncertain result when available.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DestinationPublication {
    pub account_id: String,
    pub status: PublicationOutcome,
    pub remote_post_ids: Vec<String>,
    pub error: Option<String>,
    pub segments: Vec<PublicationSegment>,
}

/// One numbered text part within a destination's publication ledger.
///
/// `index` is zero-based. A segment is claimed before its provider request,
/// then completed with a remote ID or an error outcome.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PublicationSegment {
    pub index: u64,
    pub status: PublicationOutcome,
    pub remote_post_id: Option<String>,
    pub error: Option<String>,
}

/// Durable ledger for publishing one snapshot of a draft to its destinations.
///
/// The recorded draft revision and `post` preserve what was actually sent,
/// even if the editable draft changes later. Completion is recorded after
/// destination processing reaches terminal outcomes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PublicationRecord {
    pub id: String,
    pub draft_id: String,
    pub draft_revision: u64,
    pub post: CanonicalPost,
    pub created_at_epoch_ms: i64,
    pub completed_at_epoch_ms: Option<i64>,
    pub destinations: Vec<DestinationPublication>,
}

/// Lifecycle of a scheduled publication snapshot.
///
/// A schedule is claimed before dispatch so two workers cannot publish it
/// simultaneously. A missed dispatch window enters `NeedsAttention` and
/// requires a deliberate user action.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ScheduleStatus {
    /// A durable snapshot is awaiting its due instant.
    Queued,
    /// Automatic dispatch missed the grace window and requires user action.
    NeedsAttention,
    /// Exactly one dispatcher has claimed this schedule.
    Dispatching,
    /// The claimed schedule reached a terminal publication result.
    Completed,
    /// The user cancelled the schedule before it was claimed.
    Cancelled,
}

/// Queued or completed publication of a draft snapshot at a chosen time.
///
/// `revision` guards schedule edits, while `draft_revision` identifies the
/// source draft snapshot. `scheduled_for_epoch_ms` is an absolute instant;
/// `time_zone` preserves the user's chosen display zone.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ScheduledPublication {
    pub id: String,
    pub draft_id: String,
    pub draft_revision: u64,
    /// Immutable queue-time snapshot; it does not change with later draft edits.
    pub post: CanonicalPost,
    pub revision: u64,
    pub scheduled_for_epoch_ms: i64,
    pub time_zone: String,
    pub status: ScheduleStatus,
    pub attention_reason: Option<String>,
    pub result: Option<PublicationRecord>,
    pub created_at_epoch_ms: i64,
    pub updated_at_epoch_ms: i64,
}

/// Request to queue the current revision of a draft for a future instant.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateScheduleInput {
    pub draft_id: String,
    pub scheduled_for_epoch_ms: i64,
    pub time_zone: String,
}

/// Request to change a schedule's instant and zone at a known revision.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RescheduleInput {
    pub id: String,
    pub expected_revision: u64,
    pub scheduled_for_epoch_ms: i64,
    pub time_zone: String,
}

/// Request to cancel or otherwise mutate a schedule at a known revision.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleMutationInput {
    pub id: String,
    pub expected_revision: u64,
}
