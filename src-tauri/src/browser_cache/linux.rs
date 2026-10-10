use webkit2gtk::{
    WebContextExt, WebViewExt, WebsiteDataManagerExt, WebsiteDataManagerExtManual, WebsiteDataTypes,
};

pub(super) fn size(webview: tauri::webview::PlatformWebview, completion: super::SizeCompletion) {
    let Some(manager) = webview
        .inner()
        .context()
        .and_then(|context| context.website_data_manager())
    else {
        let _ = completion.send(Err("The browser cache manager is unavailable.".to_owned()));
        return;
    };
    manager.fetch(
        WebsiteDataTypes::DISK_CACHE,
        None::<&webkit2gtk::gio::Cancellable>,
        move |result| {
            let result = result
                .map_err(|error| format!("Could not measure the browser cache: {error}"))
                .and_then(|records| {
                    records.iter().try_fold(0_u64, |bytes, record| {
                        bytes
                            .checked_add(record.size(WebsiteDataTypes::DISK_CACHE))
                            .ok_or_else(|| "Browser cache size exceeded the byte limit.".to_owned())
                    })
                })
                .map(super::SizeReport::Bytes);
            let _ = completion.send(result);
        },
    );
}

pub(super) fn clear(webview: tauri::webview::PlatformWebview, completion: super::Completion) {
    let Some(manager) = webview
        .inner()
        .context()
        .and_then(|context| context.website_data_manager())
    else {
        let _ = completion.send(Err("The browser cache manager is unavailable.".to_owned()));
        return;
    };
    manager.clear(
        WebsiteDataTypes::MEMORY_CACHE | WebsiteDataTypes::DISK_CACHE,
        webkit2gtk::glib::TimeSpan::from_seconds(0),
        None::<&webkit2gtk::gio::Cancellable>,
        move |result| {
            let _ = completion.send(
                result.map_err(|error| format!("Could not clear the browser cache: {error}")),
            );
        },
    );
}
