//! Durable publication state machine.
//!
//! Destination and segment claims move from `PENDING` to `IN_FLIGHT` exactly once. A terminal
//! result is accepted only from that claimed state; recovery converts unfinished work to
//! `UNCERTAIN` because a provider may have accepted a request before Threadline lost its result.

use super::{now_epoch_ms, Database};
use crate::{
    error::AppError,
    models::{
        CanonicalPost, DestinationPublication, PublicationOutcome, PublicationRecord,
        PublicationSegment, ScheduledPublication,
    },
};
use rusqlite::{params, OptionalExtension};

impl Database {
    pub fn begin_publication(&self, draft_id: &str) -> Result<PublicationRecord, AppError> {
        let draft = self.get_draft(draft_id)?;
        self.begin_publication_snapshot(
            &format!("draft:{}:{}", draft.id, draft.revision),
            &draft.id,
            draft.revision,
            &draft.post,
        )
    }

    pub fn begin_scheduled_publication(
        &self,
        schedule: &ScheduledPublication,
    ) -> Result<PublicationRecord, AppError> {
        self.begin_publication_snapshot(
            &format!("schedule:{}", schedule.id),
            &schedule.draft_id,
            schedule.draft_revision,
            &schedule.post,
        )
    }

    fn begin_publication_snapshot(
        &self,
        idempotency_key: &str,
        draft_id: &str,
        draft_revision: u64,
        post: &CanonicalPost,
    ) -> Result<PublicationRecord, AppError> {
        let post_json =
            serde_json::to_string(post).map_err(|error| AppError::Validation(error.to_string()))?;
        let id = uuid::Uuid::new_v4().to_string();
        let now = now_epoch_ms();
        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;
        let inserted = transaction.execute(
            "INSERT OR IGNORE INTO publication_batches(id,idempotency_key,draft_id,draft_revision,post_json,created_at_ms)
             VALUES(?1,?2,?3,?4,?5,?6)",
            params![id, idempotency_key, draft_id, draft_revision, post_json, now],
        )?;
        let batch_id = if inserted == 1 {
            for account_id in &post.destination_account_ids {
                transaction.execute(
                    "INSERT INTO publication_destinations(batch_id,account_id,status) VALUES(?1,?2,'PENDING')",
                    params![id, account_id],
                )?;
            }
            id
        } else {
            transaction.query_row(
                "SELECT id FROM publication_batches WHERE idempotency_key=?1",
                [idempotency_key],
                |row| row.get(0),
            )?
        };
        transaction.commit()?;
        drop(connection);
        self.get_publication(&batch_id)
    }

    pub fn claim_destination(&self, batch_id: &str, account_id: &str) -> Result<bool, AppError> {
        let changed = self.connection()?.execute(
            "UPDATE publication_destinations SET status='IN_FLIGHT',error=NULL
             WHERE batch_id=?1 AND account_id=?2 AND status='PENDING'",
            params![batch_id, account_id],
        )?;
        Ok(changed == 1)
    }

    pub fn finish_destination(
        &self,
        batch_id: &str,
        destination: &DestinationPublication,
    ) -> Result<(), AppError> {
        if matches!(
            destination.status,
            PublicationOutcome::Pending | PublicationOutcome::InFlight
        ) {
            return Err(AppError::Validation(
                "destination result is not terminal".into(),
            ));
        }
        let remote_ids = serde_json::to_string(&destination.remote_post_ids)
            .map_err(|error| AppError::Validation(error.to_string()))?;
        let status = outcome_name(destination.status);
        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;
        let changed = transaction.execute(
            "UPDATE publication_destinations SET status=?3,remote_ids_json=?4,error=?5
             WHERE batch_id=?1 AND account_id=?2 AND status='IN_FLIGHT'",
            params![
                batch_id,
                destination.account_id,
                status,
                remote_ids,
                destination.error
            ],
        )?;
        if changed != 1 {
            return Err(AppError::Conflict(
                "publication destination is not dispatching".into(),
            ));
        }
        let unfinished: i64 = transaction.query_row(
            "SELECT COUNT(*) FROM publication_destinations WHERE batch_id=?1 AND status IN ('PENDING','IN_FLIGHT')",
            [batch_id],
            |row| row.get(0),
        )?;
        if unfinished == 0 {
            transaction.execute(
                "UPDATE publication_batches SET completed_at_ms=?2 WHERE id=?1",
                params![batch_id, now_epoch_ms()],
            )?;
        }
        transaction.commit()?;
        Ok(())
    }

    pub fn begin_segment(
        &self,
        batch_id: &str,
        account_id: &str,
        index: usize,
    ) -> Result<bool, AppError> {
        let changed = self.connection()?.execute(
            "INSERT OR IGNORE INTO publication_segments(batch_id,account_id,segment_index,status)
             SELECT ?1,?2,?3,'IN_FLIGHT' WHERE EXISTS (
                SELECT 1 FROM publication_destinations WHERE batch_id=?1 AND account_id=?2 AND status='IN_FLIGHT'
             )",
            params![batch_id, account_id, index],
        )?;
        Ok(changed == 1)
    }

    pub fn finish_segment(
        &self,
        batch_id: &str,
        account_id: &str,
        index: usize,
        outcome: PublicationOutcome,
        remote_post_id: Option<&str>,
        error: Option<&str>,
    ) -> Result<(), AppError> {
        if !matches!(
            outcome,
            PublicationOutcome::Published
                | PublicationOutcome::Failed
                | PublicationOutcome::Uncertain
        ) {
            return Err(AppError::Validation(
                "segment result is not terminal".into(),
            ));
        }
        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;
        let changed = transaction.execute(
            "UPDATE publication_segments SET status=?4,remote_post_id=?5,error=?6
             WHERE batch_id=?1 AND account_id=?2 AND segment_index=?3 AND status='IN_FLIGHT'",
            params![
                batch_id,
                account_id,
                index,
                outcome_name(outcome),
                remote_post_id,
                error
            ],
        )?;
        if changed != 1 {
            return Err(AppError::Conflict(
                "publication segment is not dispatching".into(),
            ));
        }
        if let Some(remote_post_id) = remote_post_id {
            let remote_ids: String = transaction.query_row(
                "SELECT remote_ids_json FROM publication_destinations WHERE batch_id=?1 AND account_id=?2",
                params![batch_id, account_id],
                |row| row.get(0),
            )?;
            let mut parsed: Vec<String> = serde_json::from_str(&remote_ids)
                .map_err(|parse_error| AppError::Storage(parse_error.to_string()))?;
            parsed.push(remote_post_id.into());
            transaction.execute(
                "UPDATE publication_destinations SET remote_ids_json=?3 WHERE batch_id=?1 AND account_id=?2",
                params![batch_id, account_id, serde_json::to_string(&parsed).map_err(|serialize_error| AppError::Storage(serialize_error.to_string()))?],
            )?;
        }
        transaction.commit()?;
        Ok(())
    }

    pub fn get_publication(&self, id: &str) -> Result<PublicationRecord, AppError> {
        let connection = self.connection()?;
        let header: Option<(String, i64, String, i64, Option<i64>)> = connection
            .query_row(
                "SELECT draft_id,draft_revision,post_json,created_at_ms,completed_at_ms FROM publication_batches WHERE id=?1",
                [id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
            )
            .optional()?;
        let (draft_id, draft_revision, post_json, created_at, completed_at) =
            header.ok_or_else(|| AppError::Validation("publication not found".into()))?;
        let mut statement = connection.prepare(
            "SELECT account_id,status,remote_ids_json,error FROM publication_destinations WHERE batch_id=?1 ORDER BY rowid",
        )?;
        let destinations = statement
            .query_map([id], |row| {
                let status: String = row.get(1)?;
                let remote_ids: String = row.get(2)?;
                Ok((
                    row.get::<_, String>(0)?,
                    status,
                    remote_ids,
                    row.get::<_, Option<String>>(3)?,
                ))
            })?
            .map(|row| {
                let (account_id, status, remote_ids, error) = row?;
                let segments = load_segments(&connection, id, &account_id)?;
                Ok(DestinationPublication {
                    account_id,
                    status: parse_outcome(&status)?,
                    remote_post_ids: serde_json::from_str(&remote_ids)
                        .map_err(|parse_error| AppError::Storage(parse_error.to_string()))?,
                    error,
                    segments,
                })
            })
            .collect::<Result<Vec<_>, AppError>>()?;
        Ok(PublicationRecord {
            id: id.into(),
            draft_id,
            draft_revision: u64::try_from(draft_revision)
                .map_err(|_| AppError::Storage("publication revision is invalid".into()))?,
            post: serde_json::from_str(&post_json)
                .map_err(|error| AppError::Storage(error.to_string()))?,
            created_at_epoch_ms: created_at,
            completed_at_epoch_ms: completed_at,
            destinations,
        })
    }

    pub fn list_publications(&self) -> Result<Vec<PublicationRecord>, AppError> {
        let ids = {
            let connection = self.connection()?;
            let mut statement = connection
                .prepare("SELECT id FROM publication_batches ORDER BY created_at_ms DESC")?;
            let rows = statement
                .query_map([], |row| row.get::<_, String>(0))?
                .collect::<Result<Vec<_>, _>>()?;
            rows
        };
        ids.iter().map(|id| self.get_publication(id)).collect()
    }

    pub fn delete_publication(&self, id: &str) -> Result<(), AppError> {
        let changed = self
            .connection()?
            .execute("DELETE FROM publication_batches WHERE id=?1", [id])?;
        if changed == 0 {
            return Err(AppError::Validation("publication not found".into()));
        }
        Ok(())
    }

    pub fn recover_interrupted_publications(&self) -> Result<(), AppError> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;
        transaction.execute(
            "UPDATE publication_destinations SET status='UNCERTAIN',error='Threadline closed before the provider response was recorded' WHERE status='IN_FLIGHT'",
            [],
        )?;
        transaction.execute(
            "UPDATE publication_segments SET status='UNCERTAIN',error='Threadline closed before the provider response was recorded' WHERE status='IN_FLIGHT'",
            [],
        )?;
        transaction.execute(
            "UPDATE publication_batches SET completed_at_ms=?1 WHERE completed_at_ms IS NULL AND NOT EXISTS (
                SELECT 1 FROM publication_destinations WHERE batch_id=publication_batches.id AND status IN ('PENDING','IN_FLIGHT')
            )",
            [now_epoch_ms()],
        )?;
        transaction.commit()?;
        Ok(())
    }
}

fn load_segments(
    connection: &rusqlite::Connection,
    batch_id: &str,
    account_id: &str,
) -> Result<Vec<PublicationSegment>, AppError> {
    let mut statement = connection.prepare(
        "SELECT segment_index,status,remote_post_id,error FROM publication_segments
         WHERE batch_id=?1 AND account_id=?2 ORDER BY segment_index",
    )?;
    let segments = statement
        .query_map(params![batch_id, account_id], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, Option<String>>(3)?,
            ))
        })?
        .map(|row| {
            let (index, status, remote_post_id, error) = row?;
            Ok::<PublicationSegment, AppError>(PublicationSegment {
                index: u64::try_from(index)
                    .map_err(|_| AppError::Storage("segment index is invalid".into()))?,
                status: parse_outcome(&status)?,
                remote_post_id,
                error,
            })
        })
        .collect::<Result<Vec<_>, AppError>>()?;
    Ok(segments)
}

fn outcome_name(outcome: PublicationOutcome) -> &'static str {
    match outcome {
        PublicationOutcome::Pending => "PENDING",
        PublicationOutcome::InFlight => "IN_FLIGHT",
        PublicationOutcome::Published => "PUBLISHED",
        PublicationOutcome::Failed => "FAILED",
        PublicationOutcome::Uncertain => "UNCERTAIN",
        PublicationOutcome::Blocked => "BLOCKED",
    }
}

fn parse_outcome(value: &str) -> Result<PublicationOutcome, AppError> {
    match value {
        "PENDING" => Ok(PublicationOutcome::Pending),
        "IN_FLIGHT" => Ok(PublicationOutcome::InFlight),
        "PUBLISHED" => Ok(PublicationOutcome::Published),
        "FAILED" => Ok(PublicationOutcome::Failed),
        "UNCERTAIN" => Ok(PublicationOutcome::Uncertain),
        "BLOCKED" => Ok(PublicationOutcome::Blocked),
        _ => Err(AppError::Storage("publication status is invalid".into())),
    }
}
