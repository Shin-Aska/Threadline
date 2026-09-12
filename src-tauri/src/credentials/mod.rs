use crate::error::AppError;
pub trait CredentialStore: Send + Sync {
    fn set(&self, account_id: &str, secret: &str) -> Result<(), AppError>;
    fn get(&self, account_id: &str) -> Result<String, AppError>;
    fn delete(&self, account_id: &str) -> Result<(), AppError>;
}
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
