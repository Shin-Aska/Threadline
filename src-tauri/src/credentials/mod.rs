//! Account-scoped secret storage used when restoring provider connections.

use crate::error::AppError;
/// Persist, retrieve, and remove secrets by Threadline account ID.
///
/// Providers use this interface rather than storing tokens in account rows.
/// Implementations must be safe to share across application tasks.
pub trait CredentialStore: Send + Sync {
    /// Stores or replaces the secret for `account_id`.
    fn set(&self, account_id: &str, secret: &str) -> Result<(), AppError>;
    /// Retrieves the secret for `account_id`.
    fn get(&self, account_id: &str) -> Result<String, AppError>;
    /// Removes the secret for `account_id`.
    fn delete(&self, account_id: &str) -> Result<(), AppError>;
}
/// Operating-system keyring implementation of [`CredentialStore`].
///
/// Entries use `social.threadline.app` as their service and the account ID
/// as their key, so secrets are outside the SQLite workspace database.
pub struct OsKeychainCredentialStore;
impl CredentialStore for OsKeychainCredentialStore {
    fn set(&self, id: &str, secret: &str) -> Result<(), AppError> {
        keyring::Entry::new("social.threadline.app", id)
            .map_err(|e| AppError::Credential(e.to_string()))?
            .set_password(secret)
            .map_err(|e| AppError::Credential(e.to_string()))
    }
    fn get(&self, id: &str) -> Result<String, AppError> {
        keyring::Entry::new("social.threadline.app", id)
            .map_err(|e| AppError::Credential(e.to_string()))?
            .get_password()
            .map_err(|e| AppError::Credential(e.to_string()))
    }
    fn delete(&self, id: &str) -> Result<(), AppError> {
        keyring::Entry::new("social.threadline.app", id)
            .map_err(|e| AppError::Credential(e.to_string()))?
            .delete_credential()
            .map_err(|e| AppError::Credential(e.to_string()))
    }
}
