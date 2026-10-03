//! External website launch through the operating system's default browser.
use crate::error::AppError;

fn website_url(value: &str) -> Result<url::Url, AppError> {
    let url = url::Url::parse(value)
        .map_err(|_| AppError::Validation("External link must be a valid HTTP(S) URL".into()))?;
    if !matches!(url.scheme(), "http" | "https")
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
    {
        return Err(AppError::Validation(
            "External link must be an HTTP(S) website without embedded credentials".into(),
        ));
    }
    Ok(url)
}

#[tauri::command]
/// Opens a validated website in the default system browser.
pub fn open_external_url(url: String) -> Result<(), AppError> {
    let url = website_url(&url)?;
    webbrowser::open(url.as_str()).map_err(|error| AppError::Browser(error.to_string()))
}

#[cfg(test)]
mod tests {
    use super::website_url;

    #[test]
    fn external_links_accept_websites_with_paths_queries_and_fragments() {
        for value in [
            "https://example.com/post?search=hello%20world#reply",
            "http://localhost:8080/test",
        ] {
            assert_eq!(website_url(value).unwrap().as_str(), value);
        }
    }

    #[test]
    fn external_links_reject_non_web_targets_and_embedded_credentials() {
        for value in [
            "javascript:alert(1)",
            "file:///etc/passwd",
            "mailto:user@example.com",
            "threadline:/oauth/callback",
            "https://user:password@example.com/",
            "https://user@example.com/",
            "not a URL",
        ] {
            assert!(website_url(value).is_err(), "accepted {value}");
        }
    }
}
