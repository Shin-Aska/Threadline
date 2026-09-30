//! Browser-opening boundary shared by production logins and browser substitutes.

use super::OAuthError;

/// Opens an authorization URL using a browser selected by the caller.
pub trait BrowserOpener: Send + Sync {
    /// Opens `url`, returning a browser error when launch fails.
    fn open(&self, url: &str) -> Result<(), OAuthError>;
}

/// Opens authorization URLs with the operating system's default browser.
pub struct SystemBrowser;

impl BrowserOpener for SystemBrowser {
    fn open(&self, url: &str) -> Result<(), OAuthError> {
        webbrowser::open(url).map_err(|error| OAuthError::Browser(error.to_string()))
    }
}
