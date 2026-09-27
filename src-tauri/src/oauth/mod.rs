//! OAuth login flows for Mastodon and Bluesky.
//!
//! This module owns browser authorization, callback delivery, cancellation, and
//! provider credentials. Tauri commands turn completed credentials into accounts.

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
/// Failures reported while configuring, completing, or using an OAuth login.
pub enum OAuthError {
    /// A required flow setting or active flow is missing or invalid.
    #[error("OAuth configuration is incomplete: {0}")]
    Configuration(String),
    /// Published OAuth client metadata violates the required client contract.
    #[error("invalid OAuth client metadata: {0}")]
    InvalidMetadata(String),
    /// The callback arrived at a path other than the registered redirect path.
    #[error("the OAuth callback path did not match the active login")]
    CallbackPath,
    /// The callback state differs from the state issued for this login.
    #[error("the OAuth callback state did not match the active login")]
    StateMismatch,
    /// The provider returned an OAuth error in its callback.
    #[error("the OAuth provider denied the request")]
    ProviderDenied,
    /// The user cancelled the flow or its callback receiver closed.
    #[error("OAuth login was cancelled")]
    Cancelled,
    /// No callback arrived before the flow's deadline.
    #[error("OAuth login timed out")]
    Timeout,
    /// More than one active flow could receive a callback.
    #[error("OAuth login conflict: {0}")]
    Conflict(String),
    /// Opening the system browser failed.
    #[error("OAuth browser could not be opened: {0}")]
    Browser(String),
    /// Binding, accepting, or responding on the loopback listener failed.
    #[error("OAuth callback listener failed: {0}")]
    Listener(String),
    /// A provider request or token exchange failed.
    #[error("OAuth provider request failed: {0}")]
    Provider(String),
    /// A provider response or callback lacks required data or has invalid encoding.
    #[error("OAuth response was invalid: {0}")]
    InvalidResponse(String),
}

/// Default time allowed for a browser login callback.
pub const DEFAULT_LOGIN_TIMEOUT: Duration = Duration::from_secs(180);

#[cfg(test)]
mod tests;
