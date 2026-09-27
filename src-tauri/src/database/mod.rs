//! SQLite workspace storage and the root of draft and publication persistence.
//!
//! A mutex serializes access to the connection. Media files live beside the
//! database for persistent workspaces and in a temporary directory for tests.

use crate::{error::AppError, models::*};
use rusqlite::{params, Connection};
use std::{
    path::{Path, PathBuf},
    sync::Mutex,
};
pub(crate) mod notifications;
pub(crate) mod publishing;
/// Workspace database, its serialized SQLite connection, and draft media root.
///
/// Persistent instances leave media on disk when dropped. In-memory instances
/// own a temporary media directory and remove it on drop.
pub struct Database {
    connection: Mutex<Connection>,
    media_root: PathBuf,
    remove_media_on_drop: bool,
}
impl Database {
    /// Opens or creates a database and its sibling media directory.
    ///
    /// Initializes the schema, publication ledger, notification reads, and
    /// cleanup of media files no longer referenced by drafts.
    pub fn open(path: &Path) -> Result<Self, AppError> {
        let c = Connection::open(path)?;
        let media_root = path.with_extension("media");
        std::fs::create_dir_all(&media_root)
            .map_err(|error| AppError::Storage(error.to_string()))?;
        let db = Self {
            connection: Mutex::new(c),
            media_root,
            remove_media_on_drop: false,
        };
        db.initialize()?;
        Ok(db)
    }
    /// Creates an in-memory database with a temporary media directory.
    pub fn in_memory() -> Result<Self, AppError> {
        let c = Connection::open_in_memory()?;
        let media_root =
            std::env::temp_dir().join(format!("threadline-media-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&media_root)
            .map_err(|error| AppError::Storage(error.to_string()))?;
        let db = Self {
            connection: Mutex::new(c),
            media_root,
            remove_media_on_drop: true,
        };
        db.initialize()?;
        Ok(db)
    }
    /// Returns a locked connection, or a state error if its mutex is poisoned.
    pub(crate) fn connection(&self) -> Result<std::sync::MutexGuard<'_, Connection>, AppError> {
        self.connection
            .lock()
            .map_err(|_| AppError::StateUnavailable)
    }
    fn initialize(&self) -> Result<(), AppError> {
        self.connection()?.execute_batch("PRAGMA foreign_keys=ON; CREATE TABLE IF NOT EXISTS accounts (id TEXT PRIMARY KEY, provider TEXT NOT NULL, handle TEXT NOT NULL, display_name TEXT NOT NULL, instance_url TEXT, did TEXT, capabilities_json TEXT NOT NULL, settings_json TEXT NOT NULL DEFAULT '{}'); CREATE TABLE IF NOT EXISTS canonical_posts (id TEXT PRIMARY KEY, text TEXT NOT NULL, created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP); CREATE TABLE IF NOT EXISTS publications (id INTEGER PRIMARY KEY, canonical_id TEXT NOT NULL, account_id TEXT NOT NULL, status TEXT NOT NULL, remote_ids_json TEXT NOT NULL, error TEXT);")?;
        self.initialize_publishing()?;
        self.initialize_notification_reads()?;
        self.cleanup_orphaned_media()?;
        Ok(())
    }
    /// Inserts accounts that are not already present, preserving existing rows.
    pub fn seed(&self, accounts: &[Account]) -> Result<(), AppError> {
        let c = self.connection()?;
        for a in accounts {
            c.execute("INSERT OR IGNORE INTO accounts(id,provider,handle,display_name,instance_url,did,capabilities_json) VALUES(?1,?2,?3,?4,?5,?6,?7)",params![a.id,format!("{:?}",a.provider).to_uppercase(),a.handle,a.display_name,a.instance_url,a.did,serde_json::to_string(&a.capabilities).map_err(|e|AppError::Validation(e.to_string()))?])?;
        }
        Ok(())
    }
    /// Inserts an account or refreshes its public profile and capabilities.
    pub fn upsert_account(&self, account: &Account) -> Result<(), AppError> {
        self.connection()?.execute(
            "INSERT INTO accounts(id,provider,handle,display_name,instance_url,did,capabilities_json) VALUES(?1,?2,?3,?4,?5,?6,?7) ON CONFLICT(id) DO UPDATE SET provider=excluded.provider,handle=excluded.handle,display_name=excluded.display_name,instance_url=excluded.instance_url,did=excluded.did,capabilities_json=excluded.capabilities_json",
            params![account.id,format!("{:?}",account.provider).to_uppercase(),account.handle,account.display_name,account.instance_url,account.did,serde_json::to_string(&account.capabilities).map_err(|e|AppError::Validation(e.to_string()))?],
        )?;
        Ok(())
    }
    /// Removes an account row by its Threadline ID.
    pub fn delete_account(&self, account_id: &str) -> Result<(), AppError> {
        self.connection()?
            .execute("DELETE FROM accounts WHERE id=?1", [account_id])?;
        Ok(())
    }
    /// Replaces all stored account rows with the provided set.
    pub fn replace_accounts(&self, accounts: &[Account]) -> Result<(), AppError> {
        {
            self.connection()?.execute("DELETE FROM accounts", [])?;
        }
        self.seed(accounts)
    }
    /// Loads stored accounts in insertion order.
    ///
    /// Reconstructs each provider kind and capabilities from its database row.
    /// Returns an error if SQLite access or capabilities decoding fails.
    pub fn accounts(&self) -> Result<Vec<Account>, AppError> {
        let c = self.connection()?;
        let mut s=c.prepare("SELECT id,provider,handle,display_name,instance_url,did,capabilities_json FROM accounts ORDER BY rowid")?;
        let rows = s.query_map([], |r| {
            let provider: String = r.get(1)?;
            let json: String = r.get(6)?;
            Ok(Account {
                id: r.get(0)?,
                provider: if provider == "BLUESKY" {
                    ProviderKind::Bluesky
                } else {
                    ProviderKind::Mastodon
                },
                handle: r.get(2)?,
                display_name: r.get(3)?,
                instance_url: r.get(4)?,
                did: r.get(5)?,
                capabilities: serde_json::from_str(&json).map_err(|e| {
                    rusqlite::Error::FromSqlConversionFailure(
                        6,
                        rusqlite::types::Type::Text,
                        Box::new(e),
                    )
                })?,
            })
        })?;
        rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
    }
}
impl Drop for Database {
    fn drop(&mut self) {
        if self.remove_media_on_drop {
            let _ = std::fs::remove_dir_all(&self.media_root);
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn schema_stores_sanitized_accounts() {
        let db = Database::in_memory().expect("db");
        db.seed(&crate::accounts::mock_accounts()).expect("seed");
        assert_eq!(db.accounts().expect("accounts").len(), 3)
    }
}
#[cfg(test)]
mod publishing_tests;
