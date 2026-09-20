use super::OAuthError;

pub trait BrowserOpener: Send + Sync {
    fn open(&self, url: &str) -> Result<(), OAuthError>;
}

pub struct SystemBrowser;

impl BrowserOpener for SystemBrowser {
    fn open(&self, url: &str) -> Result<(), OAuthError> {
        webbrowser::open(url).map_err(|error| OAuthError::Browser(error.to_string()))
    }
}
