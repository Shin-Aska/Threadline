//! SQLite persistence for drafts, publication ledgers, and scheduled snapshots.
//!
//! A schedule stores the selected draft revision and serialized post at queue time. Later draft
//! edits or deletion therefore cannot change what a claimed schedule publishes. The ledger uses
//! durable claims and terminal outcomes so recovery can surface an interrupted provider call as
//! `UNCERTAIN` instead of issuing an unsafe retry.

mod drafts;
mod ledger;
mod schedules;

use super::Database;
use crate::error::AppError;
use rusqlite::OptionalExtension;
use std::time::{SystemTime, UNIX_EPOCH};

pub(crate) fn now_epoch_ms() -> i64 {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    i64::try_from(millis).unwrap_or(i64::MAX)
}

impl Database {
    pub(crate) fn initialize_publishing(&self) -> Result<(), AppError> {
        self.connection()?.execute_batch(
            "CREATE TABLE IF NOT EXISTS publishing_schema_migrations (
                version INTEGER PRIMARY KEY,
                applied_at_ms INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS drafts (
                id TEXT PRIMARY KEY,
                revision INTEGER NOT NULL,
                post_json TEXT NOT NULL,
                created_at_ms INTEGER NOT NULL,
                updated_at_ms INTEGER NOT NULL
            );
            CREATE TABLE IF NOT EXISTS draft_media (
                id TEXT PRIMARY KEY,
                draft_id TEXT NOT NULL REFERENCES drafts(id) ON DELETE CASCADE,
                attachment_id TEXT NOT NULL,
                file_name TEXT NOT NULL,
                ordinal INTEGER NOT NULL,
                UNIQUE(draft_id, attachment_id)
            );
            CREATE TABLE IF NOT EXISTS publication_batches (
                id TEXT PRIMARY KEY,
                idempotency_key TEXT NOT NULL UNIQUE,
                draft_id TEXT NOT NULL,
                draft_revision INTEGER NOT NULL,
                post_json TEXT NOT NULL,
                created_at_ms INTEGER NOT NULL,
                completed_at_ms INTEGER
            );
            CREATE TABLE IF NOT EXISTS publication_destinations (
                batch_id TEXT NOT NULL REFERENCES publication_batches(id) ON DELETE CASCADE,
                account_id TEXT NOT NULL,
                status TEXT NOT NULL,
                remote_ids_json TEXT NOT NULL DEFAULT '[]',
                error TEXT,
                PRIMARY KEY(batch_id, account_id)
            );
            CREATE TABLE IF NOT EXISTS publication_segments (
                batch_id TEXT NOT NULL,
                account_id TEXT NOT NULL,
                segment_index INTEGER NOT NULL,
                status TEXT NOT NULL,
                remote_post_id TEXT,
                error TEXT,
                PRIMARY KEY(batch_id, account_id, segment_index),
                FOREIGN KEY(batch_id, account_id) REFERENCES publication_destinations(batch_id, account_id) ON DELETE CASCADE
            );
            CREATE TABLE IF NOT EXISTS scheduled_publications (
                id TEXT PRIMARY KEY,
                draft_id TEXT NOT NULL,
                draft_revision INTEGER NOT NULL,
                post_json TEXT NOT NULL,
                revision INTEGER NOT NULL,
                scheduled_for_ms INTEGER NOT NULL,
                time_zone TEXT NOT NULL,
                status TEXT NOT NULL,
                attention_reason TEXT,
                publication_id TEXT REFERENCES publication_batches(id) ON DELETE SET NULL,
                created_at_ms INTEGER NOT NULL,
                updated_at_ms INTEGER NOT NULL
            );
            CREATE INDEX IF NOT EXISTS scheduled_due_idx
                ON scheduled_publications(status, scheduled_for_ms);
            INSERT OR IGNORE INTO publishing_schema_migrations(version, applied_at_ms)
                VALUES(1, CAST(strftime('%s','now') AS INTEGER) * 1000);",
        )?;
        self.migrate_publication_batch_identity()?;
        Ok(())
    }

    fn migrate_publication_batch_identity(&self) -> Result<(), AppError> {
        let connection = self.connection()?;
        let schema: Option<String> = connection
            .query_row(
                "SELECT sql FROM sqlite_master WHERE type='table' AND name='publication_batches'",
                [],
                |row| row.get(0),
            )
            .optional()?;
        let Some(schema) = schema else { return Ok(()) };
        if !schema
            .replace(' ', "")
            .contains("UNIQUE(draft_id,draft_revision)")
        {
            return Ok(());
        }
        let identity = if schema.contains("idempotency_key") {
            "idempotency_key"
        } else {
            "'draft:' || draft_id || ':' || draft_revision"
        };
        connection.execute_batch(&format!(
            "PRAGMA foreign_keys=OFF;
             BEGIN IMMEDIATE;
             CREATE TABLE publication_batches_v2 (
                id TEXT PRIMARY KEY,
                idempotency_key TEXT NOT NULL UNIQUE,
                draft_id TEXT NOT NULL,
                draft_revision INTEGER NOT NULL,
                post_json TEXT NOT NULL,
                created_at_ms INTEGER NOT NULL,
                completed_at_ms INTEGER
             );
             INSERT INTO publication_batches_v2
                SELECT id,{identity},draft_id,draft_revision,post_json,created_at_ms,completed_at_ms
                FROM publication_batches;
             DROP TABLE publication_batches;
             ALTER TABLE publication_batches_v2 RENAME TO publication_batches;
             COMMIT;
             PRAGMA foreign_keys=ON;"
        ))?;
        Ok(())
    }
}
