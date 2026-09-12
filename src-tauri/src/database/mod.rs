use crate::{error::AppError, models::*};
use rusqlite::{params, Connection};
use std::{path::Path, sync::Mutex};
pub struct Database {
    connection: Mutex<Connection>,
}
impl Database {
    pub fn open(path: &Path) -> Result<Self, AppError> {
        let c = Connection::open(path)?;
        let db = Self {
            connection: Mutex::new(c),
        };
        db.initialize()?;
        Ok(db)
    }
    pub fn in_memory() -> Result<Self, AppError> {
        let c = Connection::open_in_memory()?;
        let db = Self {
            connection: Mutex::new(c),
        };
        db.initialize()?;
        Ok(db)
    }
    fn connection(&self) -> Result<std::sync::MutexGuard<'_, Connection>, AppError> {
        self.connection
            .lock()
            .map_err(|_| AppError::StateUnavailable)
    }
    fn initialize(&self) -> Result<(), AppError> {
        self.connection()?.execute_batch("CREATE TABLE IF NOT EXISTS accounts (id TEXT PRIMARY KEY, provider TEXT NOT NULL, handle TEXT NOT NULL, display_name TEXT NOT NULL, instance_url TEXT, did TEXT, capabilities_json TEXT NOT NULL, settings_json TEXT NOT NULL DEFAULT '{}'); CREATE TABLE IF NOT EXISTS canonical_posts (id TEXT PRIMARY KEY, text TEXT NOT NULL, created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP); CREATE TABLE IF NOT EXISTS publications (id INTEGER PRIMARY KEY, canonical_id TEXT NOT NULL, account_id TEXT NOT NULL, status TEXT NOT NULL, remote_ids_json TEXT NOT NULL, error TEXT);")?;
        Ok(())
    }
    pub fn seed(&self, accounts: &[Account]) -> Result<(), AppError> {
        let c = self.connection()?;
        for a in accounts {
            c.execute("INSERT OR IGNORE INTO accounts(id,provider,handle,display_name,instance_url,did,capabilities_json) VALUES(?1,?2,?3,?4,?5,?6,?7)",params![a.id,format!("{:?}",a.provider).to_uppercase(),a.handle,a.display_name,a.instance_url,a.did,serde_json::to_string(&a.capabilities).map_err(|e|AppError::Validation(e.to_string()))?])?;
        }
        Ok(())
    }
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
