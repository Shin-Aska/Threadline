use tokio::sync::oneshot;

#[cfg(any(target_os = "windows", test))]
mod disk;

#[cfg(target_os = "linux")]
#[path = "browser_cache/linux.rs"]
mod platform;
#[cfg(target_os = "macos")]
#[path = "browser_cache/macos.rs"]
mod platform;
#[cfg(target_os = "windows")]
#[path = "browser_cache/windows.rs"]
mod platform;

#[cfg(not(any(target_os = "linux", target_os = "macos", target_os = "windows")))]
mod platform {
    pub(super) fn size(_: tauri::webview::PlatformWebview, completion: super::SizeCompletion) {
        let _ = completion.send(Ok(super::SizeReport::Unavailable));
    }

    pub(super) fn clear(_: tauri::webview::PlatformWebview, completion: super::Completion) {
        let _ = completion.send(Err(
            "Browser cache clearing is supported on desktop platforms.".to_owned(),
        ));
    }
}

type Completion = oneshot::Sender<Result<(), String>>;
type SizeCompletion = oneshot::Sender<Result<SizeReport, String>>;

enum SizeReport {
    #[cfg(target_os = "linux")]
    Bytes(u64),
    #[cfg(target_os = "windows")]
    Profile(std::path::PathBuf),
    #[cfg(not(any(target_os = "linux", target_os = "windows")))]
    Unavailable,
}

#[tauri::command]
pub async fn get_browser_cache_size(webview: tauri::Webview) -> Result<Option<u64>, String> {
    let (completion, finished) = oneshot::channel();
    webview
        .with_webview(move |platform_webview| platform::size(platform_webview, completion))
        .map_err(|error| format!("Could not access the browser cache: {error}"))?;
    let report = tokio::time::timeout(std::time::Duration::from_secs(60), finished)
        .await
        .map_err(|_| "Browser cache measurement did not finish within 60 seconds.".to_owned())?
        .map_err(|_| "Browser cache measurement ended before completion.".to_owned())??;
    match report {
        #[cfg(target_os = "linux")]
        SizeReport::Bytes(bytes) => Ok(Some(bytes)),
        #[cfg(target_os = "windows")]
        SizeReport::Profile(profile) => {
            tauri::async_runtime::spawn_blocking(move || disk::profile_cache_size(&profile))
                .await
                .map_err(|error| format!("Browser cache measurement failed: {error}"))?
                .map(Some)
                .map_err(|error| format!("Could not measure the browser cache: {error}"))
        }
        #[cfg(not(any(target_os = "linux", target_os = "windows")))]
        SizeReport::Unavailable => Ok(None),
    }
}

#[tauri::command]
pub async fn clear_browser_cache(webview: tauri::Webview) -> Result<(), String> {
    let (completion, finished) = oneshot::channel();
    webview
        .with_webview(move |platform_webview| platform::clear(platform_webview, completion))
        .map_err(|error| format!("Could not access the browser cache: {error}"))?;
    tokio::time::timeout(std::time::Duration::from_secs(60), finished)
        .await
        .map_err(|_| "Browser cache clearing did not finish within 60 seconds.".to_owned())?
        .map_err(|_| "Browser cache clearing ended before completion.".to_owned())?
}
