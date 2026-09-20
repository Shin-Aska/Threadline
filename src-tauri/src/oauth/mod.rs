pub mod bluesky;
mod browser;
pub mod callback;
pub mod commands;
pub mod config;
pub mod coordinator;
pub mod mastodon;
mod network;

use std::time::Duration;

#[derive(Debug, thiserror::Error, PartialEq, Eq)]
pub enum OAuthError {
    #[error("OAuth configuration is incomplete: {0}")]
    Configuration(String),
    #[error("invalid OAuth client metadata: {0}")]
    InvalidMetadata(String),
    #[error("the OAuth callback path did not match the active login")]
    CallbackPath,
    #[error("the OAuth callback state did not match the active login")]
    StateMismatch,
    #[error("the OAuth provider denied the request")]
    ProviderDenied,
    #[error("OAuth login was cancelled")]
    Cancelled,
    #[error("OAuth login timed out")]
    Timeout,
    #[error("OAuth login conflict: {0}")]
    Conflict(String),
    #[error("OAuth browser could not be opened: {0}")]
    Browser(String),
    #[error("OAuth callback listener failed: {0}")]
    Listener(String),
    #[error("OAuth provider request failed: {0}")]
    Provider(String),
    #[error("OAuth response was invalid: {0}")]
    InvalidResponse(String),
}

pub const DEFAULT_LOGIN_TIMEOUT: Duration = Duration::from_secs(180);

#[cfg(test)]
mod tests;
