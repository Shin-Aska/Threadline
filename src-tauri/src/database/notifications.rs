use std::collections::HashSet;

use rusqlite::params;

use crate::error::AppError;

use super::Database;

impl Database {
    pub(super) fn initialize_notification_reads(&self) -> Result<(), AppError> {
        self.connection()?.execute_batch(
            "CREATE TABLE IF NOT EXISTS notification_reads (
                account_id TEXT NOT NULL,
                notification_id TEXT NOT NULL,
                read_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
                PRIMARY KEY (account_id, notification_id),
                FOREIGN KEY (account_id) REFERENCES accounts(id) ON DELETE CASCADE
            );",
        )?;
        Ok(())
    }

    pub fn mark_notifications_read_local(
        &self,
        account_id: &str,
        notification_ids: &[String],
    ) -> Result<(), AppError> {
        let mut connection = self.connection()?;
        let transaction = connection.transaction()?;
        for notification_id in notification_ids {
            transaction.execute(
                "INSERT OR IGNORE INTO notification_reads(account_id, notification_id)
                 VALUES(?1, ?2)",
                params![account_id, notification_id],
            )?;
        }
        transaction.commit()?;
        Ok(())
    }

    pub fn notification_read_ids(&self, account_id: &str) -> Result<HashSet<String>, AppError> {
        let connection = self.connection()?;
        let mut statement = connection
            .prepare("SELECT notification_id FROM notification_reads WHERE account_id=?1")?;
        let rows = statement.query_map([account_id], |row| row.get(0))?;
        rows.collect::<Result<HashSet<_>, _>>().map_err(Into::into)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn notification_read_ids_survive_database_reopen_and_are_account_scoped() {
        let path = std::env::temp_dir().join(format!(
            "threadline-notification-reads-{}.sqlite",
            uuid::Uuid::new_v4()
        ));
        let database = Database::open(&path).expect("database");
        database
            .seed(&crate::accounts::mock_accounts())
            .expect("accounts");
        let account_id = &crate::accounts::mock_accounts()[1].id;

        database
            .mark_notifications_read_local(account_id, &["notification-1".into()])
            .expect("mark read");
        drop(database);

        let database = Database::open(&path).expect("reopen database");
        let read = database
            .notification_read_ids(account_id)
            .expect("read ids");
        assert!(read.contains("notification-1"));
        assert!(database
            .notification_read_ids("another-account")
            .expect("other account")
            .is_empty());
        drop(database);
        std::fs::remove_file(&path).expect("remove database");
        std::fs::remove_dir_all(path.with_extension("media")).expect("remove media directory");
    }
}
