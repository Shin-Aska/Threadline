use std::{cell::RefCell, rc::Rc};

use objc2::{rc::Retained, MainThreadMarker};
use objc2_foundation::{NSDate, NSSet};
use objc2_web_kit::{WKWebView, WKWebsiteDataTypeDiskCache, WKWebsiteDataTypeMemoryCache};

pub(super) fn size(_: tauri::webview::PlatformWebview, completion: super::SizeCompletion) {
    let _ = completion.send(Ok(super::SizeReport::Unavailable));
}

pub(super) fn clear(webview: tauri::webview::PlatformWebview, completion: super::Completion) {
    let Some(_main_thread) = MainThreadMarker::new() else {
        let _ = completion.send(Err("The browser cache requires the main thread.".to_owned()));
        return;
    };
    // SAFETY: Tauri exposes a live WKWebView pointer only inside with_webview on
    // the main thread; retaining it keeps it alive throughout the native calls.
    let Some(view) = (unsafe { Retained::retain(webview.inner().cast::<WKWebView>()) }) else {
        let _ = completion.send(Err("The browser webview is unavailable.".to_owned()));
        return;
    };
    // SAFETY: The retained WKWebView is accessed on the main thread and its
    // configuration getter returns an owned Objective-C object.
    let configuration = unsafe { view.configuration() };
    // SAFETY: The retained configuration owns a valid website data store, and
    // this getter is invoked on the main thread required by WebKit.
    let store = unsafe { configuration.websiteDataStore() };
    // SAFETY: WebKit initializes this NSString cache-type constant before use.
    let disk_cache = unsafe { WKWebsiteDataTypeDiskCache };
    // SAFETY: WebKit initializes this NSString cache-type constant before use.
    let memory_cache = unsafe { WKWebsiteDataTypeMemoryCache };
    let types = NSSet::from_slice(&[disk_cache, memory_cache]);
    let completion = Rc::new(RefCell::new(Some(completion)));
    let handler = block2::RcBlock::new(move || {
        if let Some(sender) = completion.borrow_mut().take() {
            let _ = sender.send(Ok(()));
        }
    });
    // SAFETY: Both arguments are retained native objects on the main thread;
    // WebKit copies the block and invokes it after the selected data is removed.
    unsafe {
        store.removeDataOfTypes_modifiedSince_completionHandler(
            &types,
            &NSDate::distantPast(),
            &handler,
        )
    };
}
