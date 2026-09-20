use super::{now_epoch_ms, Database};
use crate::{
    error::AppError,
    models::{CanonicalPost, DraftRecord, SaveDraftInput},
};
use base64::{engine::general_purpose::STANDARD, Engine};
use rusqlite::{params, OptionalExtension};

impl Database {
    pub(crate) fn cleanup_orphaned_media(&self) -> Result<(), AppError> {
        let referenced = {
            let connection = self.connection()?;
            let mut statement = connection.prepare("SELECT file_name FROM draft_media")?;
            let names = statement
                .query_map([], |row| row.get::<_, String>(0))?
                .collect::<Result<std::collections::HashSet<_>, _>>()?;
            names
        };
        let entries = std::fs::read_dir(&self.media_root)
            .map_err(|error| AppError::Storage(error.to_string()))?;
        for entry in entries {
            let entry = entry.map_err(|error| AppError::Storage(error.to_string()))?;
            let file_name = entry.file_name().to_string_lossy().into_owned();
            if !referenced.contains(&file_name) && uuid::Uuid::parse_str(&file_name).is_ok() {
                std::fs::remove_file(entry.path())
                    .map_err(|error| AppError::Storage(error.to_string()))?;
            }
        }
        Ok(())
    }

    pub fn save_draft(&self, input: SaveDraftInput) -> Result<DraftRecord, AppError> {
        if input
            .id
            .as_deref()
            .is_some_and(|id| uuid::Uuid::parse_str(id).is_err())
        {
            return Err(AppError::Validation("draft ID is invalid".into()));
        }
        let id = input.id.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let now = now_epoch_ms();
        let mut stored_post = input.post.clone();
        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;
        let existing: Option<(i64, i64)> = transaction
            .query_row(
                "SELECT revision, created_at_ms FROM drafts WHERE id=?1",
                [&id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()?;
        let (revision, created_at) = match existing {
            Some((revision, created_at)) => {
                let expected = input.expected_revision.ok_or_else(|| {
                    AppError::Conflict("expectedRevision is required when updating a draft".into())
                })?;
                if i64::try_from(expected).ok() != Some(revision) {
                    return Err(AppError::Conflict("draft changed in another editor".into()));
                }
                (revision + 1, created_at)
            }
            None => {
                if input.expected_revision.is_some() {
                    return Err(AppError::Conflict("draft no longer exists".into()));
                }
                (1, now)
            }
        };
        let files = persist_media_files(self, &stored_post)?;
        for attachment in &mut stored_post.media {
            attachment.data_base64.clear();
        }
        let post_json = serde_json::to_string(&stored_post)
            .map_err(|error| AppError::Validation(error.to_string()))?;
        let old_files = media_file_names(&transaction, &id)?;
        transaction.execute(
            "INSERT INTO drafts(id,revision,post_json,created_at_ms,updated_at_ms)
             VALUES(?1,?2,?3,?4,?5)
             ON CONFLICT(id) DO UPDATE SET revision=excluded.revision,post_json=excluded.post_json,updated_at_ms=excluded.updated_at_ms",
            params![id, revision, post_json, created_at, now],
        )?;
        transaction.execute("DELETE FROM draft_media WHERE draft_id=?1", [&id])?;
        for (ordinal, (attachment_id, file_name)) in files.iter().enumerate() {
            transaction.execute(
                "INSERT INTO draft_media(id,draft_id,attachment_id,file_name,ordinal) VALUES(?1,?2,?3,?4,?5)",
                params![uuid::Uuid::new_v4().to_string(), id, attachment_id, file_name, ordinal],
            )?;
        }
        transaction.commit()?;
        drop(connection);
        remove_media_files(self, &old_files);
        self.get_draft(&id)
    }

    pub fn get_draft(&self, id: &str) -> Result<DraftRecord, AppError> {
        let connection = self.connection()?;
        let (revision, post_json, created_at, updated_at): (i64, String, i64, i64) = connection
            .query_row(
                "SELECT revision,post_json,created_at_ms,updated_at_ms FROM drafts WHERE id=?1",
                [id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
            )
            .optional()?
            .ok_or_else(|| AppError::Validation("draft not found".into()))?;
        let mut post: CanonicalPost = serde_json::from_str(&post_json)
            .map_err(|error| AppError::Storage(error.to_string()))?;
        let mut statement = connection.prepare(
            "SELECT attachment_id,file_name FROM draft_media WHERE draft_id=?1 ORDER BY ordinal",
        )?;
        let files = statement
            .query_map([id], |row| {
                Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
            })?
            .collect::<Result<Vec<_>, _>>()?;
        drop(statement);
        drop(connection);
        for attachment in &mut post.media {
            let file_name = files
                .iter()
                .find(|(attachment_id, _)| attachment_id == &attachment.id)
                .map(|(_, file_name)| file_name)
                .ok_or_else(|| AppError::Storage("draft attachment file is missing".into()))?;
            let bytes = std::fs::read(media_path(self, file_name)?)
                .map_err(|error| AppError::Storage(error.to_string()))?;
            attachment.data_base64 = STANDARD.encode(bytes);
        }
        Ok(DraftRecord {
            id: id.into(),
            revision: u64::try_from(revision)
                .map_err(|_| AppError::Storage("draft revision is invalid".into()))?,
            post,
            created_at_epoch_ms: created_at,
            updated_at_epoch_ms: updated_at,
        })
    }

    pub fn list_drafts(&self) -> Result<Vec<DraftRecord>, AppError> {
        let ids = {
            let connection = self.connection()?;
            let mut statement =
                connection.prepare("SELECT id FROM drafts ORDER BY updated_at_ms DESC")?;
            let rows = statement
                .query_map([], |row| row.get::<_, String>(0))?
                .collect::<Result<Vec<_>, _>>()?;
            rows
        };
        ids.iter().map(|id| self.get_draft(id)).collect()
    }

    pub fn delete_draft(&self, id: &str) -> Result<(), AppError> {
        let old_files = {
            let connection = self.connection()?;
            let files = media_file_names(&connection, id)?;
            let changed = connection.execute("DELETE FROM drafts WHERE id=?1", [id])?;
            if changed == 0 {
                return Err(AppError::Validation("draft not found".into()));
            }
            files
        };
        remove_media_files(self, &old_files);
        Ok(())
    }
}

fn persist_media_files(
    database: &Database,
    post: &CanonicalPost,
) -> Result<Vec<(String, String)>, AppError> {
    post.media
        .iter()
        .map(|attachment| {
            let bytes = STANDARD
                .decode(&attachment.data_base64)
                .map_err(|_| AppError::Validation("attachment data is not valid base64".into()))?;
            if bytes.len() != attachment.size_bytes {
                return Err(AppError::Validation(
                    "attachment size does not match its content".into(),
                ));
            }
            let file_name = uuid::Uuid::new_v4().to_string();
            std::fs::write(database.media_root.join(&file_name), bytes)
                .map_err(|error| AppError::Storage(error.to_string()))?;
            Ok((attachment.id.clone(), file_name))
        })
        .collect()
}

fn media_file_names(connection: &rusqlite::Connection, id: &str) -> Result<Vec<String>, AppError> {
    let mut statement =
        connection.prepare("SELECT file_name FROM draft_media WHERE draft_id=?1")?;
    let names = statement
        .query_map([id], |row| row.get::<_, String>(0))?
        .collect::<Result<Vec<_>, _>>()?;
    Ok(names)
}

fn remove_media_files(database: &Database, file_names: &[String]) {
    for file_name in file_names {
        if let Ok(path) = media_path(database, file_name) {
            let _ = std::fs::remove_file(path);
        }
    }
}

fn media_path(database: &Database, file_name: &str) -> Result<std::path::PathBuf, AppError> {
    uuid::Uuid::parse_str(file_name)
        .map_err(|_| AppError::Storage("draft attachment path is invalid".into()))?;
    Ok(database.media_root.join(file_name))
}
