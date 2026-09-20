//! Serializable contracts for durable drafting, publishing, and scheduling.

use super::CanonicalPost;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SaveDraftInput {
    pub id: Option<String>,
    pub expected_revision: Option<u64>,
    pub post: CanonicalPost,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DraftRecord {
    pub id: String,
    pub revision: u64,
    pub post: CanonicalPost,
    pub created_at_epoch_ms: i64,
    pub updated_at_epoch_ms: i64,
}

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DestinationPublication {
    pub account_id: String,
    pub status: PublicationOutcome,
    pub remote_post_ids: Vec<String>,
    pub error: Option<String>,
    pub segments: Vec<PublicationSegment>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PublicationSegment {
    pub index: u64,
    pub status: PublicationOutcome,
    pub remote_post_id: Option<String>,
    pub error: Option<String>,
}

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

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CreateScheduleInput {
    pub draft_id: String,
    pub scheduled_for_epoch_ms: i64,
    pub time_zone: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct RescheduleInput {
    pub id: String,
    pub expected_revision: u64,
    pub scheduled_for_epoch_ms: i64,
    pub time_zone: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleMutationInput {
    pub id: String,
    pub expected_revision: u64,
}
