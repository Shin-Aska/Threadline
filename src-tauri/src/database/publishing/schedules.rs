//! Persisted schedule state transitions and their optimistic-concurrency guards.
//!
//! The automatic dispatcher may claim only a queued item that is due within 60 seconds. Older
//! items move to `NEEDS_ATTENTION`, requiring an explicit reschedule or send-now action.

use super::{now_epoch_ms, Database};
use crate::{
    error::AppError,
    models::{
        CreateScheduleInput, PublicationOutcome, PublicationRecord, RescheduleInput,
        ScheduleStatus, ScheduledPublication,
    },
};
use rusqlite::{params, OptionalExtension};

const AUTOMATIC_DISPATCH_GRACE_MS: i64 = 60_000;

type ScheduleRow = (
    String,
    i64,
    String,
    i64,
    i64,
    String,
    String,
    Option<String>,
    Option<String>,
    i64,
    i64,
);

impl Database {
    pub fn create_schedule(
        &self,
        input: CreateScheduleInput,
    ) -> Result<ScheduledPublication, AppError> {
        let draft = self.get_draft(&input.draft_id)?;
        validate_time_zone(&input.time_zone)?;
        let id = uuid::Uuid::new_v4().to_string();
        let now = now_epoch_ms();
        let post_json = serde_json::to_string(&draft.post)
            .map_err(|error| AppError::Validation(error.to_string()))?;
        self.connection()?.execute(
            "INSERT INTO scheduled_publications(id,draft_id,draft_revision,post_json,revision,scheduled_for_ms,time_zone,status,created_at_ms,updated_at_ms)
             VALUES(?1,?2,?3,?4,1,?5,?6,'QUEUED',?7,?7)",
            params![id, input.draft_id, draft.revision, post_json, input.scheduled_for_epoch_ms, input.time_zone, now],
        )?;
        self.get_schedule(&id)
    }

    pub fn get_schedule(&self, id: &str) -> Result<ScheduledPublication, AppError> {
        let row: Option<ScheduleRow> = self
            .connection()?
            .query_row(
                "SELECT draft_id,draft_revision,post_json,revision,scheduled_for_ms,time_zone,status,attention_reason,publication_id,created_at_ms,updated_at_ms FROM scheduled_publications WHERE id=?1",
                [id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?, row.get(7)?, row.get(8)?, row.get(9)?, row.get(10)?)),
            )
            .optional()?;
        let (
            draft_id,
            draft_revision,
            post_json,
            revision,
            scheduled_for,
            time_zone,
            status,
            attention_reason,
            publication_id,
            created_at,
            updated_at,
        ) = row.ok_or_else(|| AppError::Validation("scheduled publication not found".into()))?;
        let result = publication_id
            .as_deref()
            .map(|publication_id| self.get_publication(publication_id))
            .transpose()?;
        Ok(ScheduledPublication {
            id: id.into(),
            draft_id,
            draft_revision: u64::try_from(draft_revision)
                .map_err(|_| AppError::Storage("scheduled draft revision is invalid".into()))?,
            post: serde_json::from_str(&post_json)
                .map_err(|error| AppError::Storage(error.to_string()))?,
            revision: u64::try_from(revision)
                .map_err(|_| AppError::Storage("schedule revision is invalid".into()))?,
            scheduled_for_epoch_ms: scheduled_for,
            time_zone,
            status: parse_schedule_status(&status)?,
            attention_reason,
            result,
            created_at_epoch_ms: created_at,
            updated_at_epoch_ms: updated_at,
        })
    }

    pub fn list_schedules(&self) -> Result<Vec<ScheduledPublication>, AppError> {
        let ids = {
            let connection = self.connection()?;
            let mut statement = connection
                .prepare("SELECT id FROM scheduled_publications ORDER BY scheduled_for_ms ASC")?;
            let rows = statement
                .query_map([], |row| row.get::<_, String>(0))?
                .collect::<Result<Vec<_>, _>>()?;
            rows
        };
        ids.iter().map(|id| self.get_schedule(id)).collect()
    }

    pub fn reschedule(&self, input: RescheduleInput) -> Result<ScheduledPublication, AppError> {
        validate_time_zone(&input.time_zone)?;
        let changed = self.connection()?.execute(
            "UPDATE scheduled_publications
             SET revision=revision+1,scheduled_for_ms=?3,time_zone=?4,status='QUEUED',attention_reason=NULL,publication_id=NULL,updated_at_ms=?5
             WHERE id=?1 AND revision=?2 AND status IN ('QUEUED','NEEDS_ATTENTION')",
            params![input.id, input.expected_revision, input.scheduled_for_epoch_ms, input.time_zone, now_epoch_ms()],
        )?;
        if changed != 1 {
            return Err(AppError::Conflict(
                "schedule changed or can no longer be edited".into(),
            ));
        }
        self.get_schedule(&input.id)
    }

    pub fn cancel_schedule(
        &self,
        id: &str,
        expected_revision: u64,
    ) -> Result<ScheduledPublication, AppError> {
        let changed = self.connection()?.execute(
            "UPDATE scheduled_publications SET revision=revision+1,status='CANCELLED',attention_reason=NULL,updated_at_ms=?3
             WHERE id=?1 AND revision=?2 AND status IN ('QUEUED','NEEDS_ATTENTION')",
            params![id, expected_revision, now_epoch_ms()],
        )?;
        if changed != 1 {
            return Err(AppError::Conflict(
                "schedule changed or can no longer be cancelled".into(),
            ));
        }
        self.get_schedule(id)
    }

    pub fn claim_schedule_now(
        &self,
        id: &str,
        expected_revision: u64,
    ) -> Result<Option<ScheduledPublication>, AppError> {
        let changed = self.connection()?.execute(
            "UPDATE scheduled_publications SET revision=revision+1,status='DISPATCHING',attention_reason=NULL,updated_at_ms=?3
             WHERE id=?1 AND revision=?2 AND status IN ('QUEUED','NEEDS_ATTENTION')",
            params![id, expected_revision, now_epoch_ms()],
        )?;
        if changed == 1 {
            self.get_schedule(id).map(Some)
        } else {
            Ok(None)
        }
    }

    pub fn claim_due_schedule(&self, now: i64) -> Result<Option<ScheduledPublication>, AppError> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;
        let stale_before = now.saturating_sub(AUTOMATIC_DISPATCH_GRACE_MS);
        transaction.execute(
            "UPDATE scheduled_publications
             SET revision=revision+1,status='NEEDS_ATTENTION',attention_reason='Scheduled time was missed by more than 1 minute while Threadline was running',updated_at_ms=?1
             WHERE status='QUEUED' AND scheduled_for_ms<?2",
            params![now, stale_before],
        )?;
        let id: Option<String> = transaction
            .query_row(
                "SELECT id FROM scheduled_publications WHERE status='QUEUED' AND scheduled_for_ms<=?1 ORDER BY scheduled_for_ms LIMIT 1",
                [now],
                |row| row.get(0),
            )
            .optional()?;
        let claimed = match id {
            Some(id) => {
                let changed = transaction.execute(
                    "UPDATE scheduled_publications SET revision=revision+1,status='DISPATCHING',updated_at_ms=?2 WHERE id=?1 AND status='QUEUED'",
                    params![id, now],
                )?;
                (changed == 1).then_some(id)
            }
            None => None,
        };
        transaction.commit()?;
        drop(connection);
        claimed
            .as_deref()
            .map(|id| self.get_schedule(id))
            .transpose()
    }

    pub fn complete_schedule(
        &self,
        id: &str,
        publication: &PublicationRecord,
    ) -> Result<ScheduledPublication, AppError> {
        let all_published = publication
            .destinations
            .iter()
            .all(|destination| destination.status == PublicationOutcome::Published);
        let (status, reason) = if all_published {
            ("COMPLETED", None)
        } else {
            (
                "NEEDS_ATTENTION",
                Some("One or more destinations need review"),
            )
        };
        let changed = self.connection()?.execute(
            "UPDATE scheduled_publications SET revision=revision+1,status=?2,attention_reason=?3,publication_id=?4,updated_at_ms=?5
             WHERE id=?1 AND status='DISPATCHING'",
            params![id, status, reason, publication.id, now_epoch_ms()],
        )?;
        if changed != 1 {
            return Err(AppError::Conflict("schedule is not dispatching".into()));
        }
        self.get_schedule(id)
    }

    pub fn fail_schedule(&self, id: &str, reason: &str) -> Result<ScheduledPublication, AppError> {
        let changed = self.connection()?.execute(
            "UPDATE scheduled_publications SET revision=revision+1,status='NEEDS_ATTENTION',attention_reason=?2,updated_at_ms=?3
             WHERE id=?1 AND status='DISPATCHING'",
            params![id, reason, now_epoch_ms()],
        )?;
        if changed != 1 {
            return Err(AppError::Conflict("schedule is not dispatching".into()));
        }
        self.get_schedule(id)
    }

    pub fn mark_startup_missed(&self, now: i64) -> Result<(), AppError> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;
        transaction.execute(
            "UPDATE scheduled_publications SET revision=revision+1,status='NEEDS_ATTENTION',attention_reason='Scheduled time passed while Threadline was closed',updated_at_ms=?1
             WHERE status='QUEUED' AND scheduled_for_ms<=?1",
            [now],
        )?;
        transaction.execute(
            "UPDATE scheduled_publications SET revision=revision+1,status='NEEDS_ATTENTION',attention_reason='Threadline closed while dispatch was in progress',updated_at_ms=?1
             WHERE status='DISPATCHING'",
            [now],
        )?;
        transaction.commit()?;
        Ok(())
    }
}

fn parse_schedule_status(status: &str) -> Result<ScheduleStatus, AppError> {
    match status {
        "QUEUED" => Ok(ScheduleStatus::Queued),
        "NEEDS_ATTENTION" => Ok(ScheduleStatus::NeedsAttention),
        "DISPATCHING" => Ok(ScheduleStatus::Dispatching),
        "COMPLETED" => Ok(ScheduleStatus::Completed),
        "CANCELLED" => Ok(ScheduleStatus::Cancelled),
        _ => Err(AppError::Storage("schedule status is invalid".into())),
    }
}

fn validate_time_zone(value: &str) -> Result<(), AppError> {
    jiff::tz::TimeZone::get(value)
        .map(|_| ())
        .map_err(|_| AppError::Validation("time zone is not a valid IANA identifier".into()))
}
