use std::{cell::RefCell, rc::Rc};
use webview2_com::{
    CallDevToolsProtocolMethodCompletedHandler, ClearBrowsingDataCompletedHandler, CoTaskMemPWSTR,
    Microsoft::Web::WebView2::Win32::{
        ICoreWebView2Profile2, ICoreWebView2_13, COREWEBVIEW2_BROWSING_DATA_KINDS_DISK_CACHE,
    },
};
use windows_core::{w, Interface, PWSTR};

pub(super) fn size(webview: tauri::webview::PlatformWebview, completion: super::SizeCompletion) {
    let result = (|| -> windows_core::Result<String> {
        // SAFETY: Tauri invokes this function on the owning WebView2 apartment;
        // the controller and queried interfaces retain the live COM objects.
        let core = unsafe { webview.controller().CoreWebView2() }?;
        let version = core.cast::<ICoreWebView2_13>()?;
        // SAFETY: The retained interface is used on its owning apartment thread.
        let profile = unsafe { version.Profile() }?;
        let mut raw_path = PWSTR::null();
        // SAFETY: The initialized out pointer is valid for this synchronous call.
        // WebView2 allocates the returned UTF-16 string with CoTaskMemAlloc.
        let status = unsafe { profile.ProfilePath(&mut raw_path) };
        let path = CoTaskMemPWSTR::from(raw_path);
        status?;
        Ok(path.to_string())
    })();
    let result = result
        .map_err(|error| format!("Could not locate the browser cache: {error}"))
        .and_then(|path| {
            if path.is_empty() {
                Err("The browser profile path is unavailable.".to_owned())
            } else {
                Ok(super::SizeReport::Profile(std::path::PathBuf::from(path)))
            }
        });
    let _ = completion.send(result);
}

pub(super) fn clear(webview: tauri::webview::PlatformWebview, completion: super::Completion) {
    let completion = Rc::new(RefCell::new(Some(completion)));
    let result = (|| -> windows_core::Result<()> {
        // SAFETY: Tauri invokes this closure on the WebView2 apartment thread;
        // the cloned controller retains its live COM interface for this call.
        let core = unsafe { webview.controller().CoreWebView2() }?;
        let version = core.cast::<ICoreWebView2_13>()?;
        // SAFETY: `version` is a retained interface queried from the live webview
        // on its owning apartment thread; Profile has no caller-owned out pointer.
        let profile = unsafe { version.Profile() }?.cast::<ICoreWebView2Profile2>()?;
        let disk_finished = Rc::clone(&completion);
        let handler = ClearBrowsingDataCompletedHandler::create(Box::new(move |status| {
            let result = (|| -> windows_core::Result<()> {
                status?;
                let memory_finished = Rc::clone(&disk_finished);
                let handler = CallDevToolsProtocolMethodCompletedHandler::create(Box::new(
                    move |status, _| {
                        finish(&memory_finished, status);
                        Ok(())
                    },
                ));
                // SAFETY: WebView2 calls this callback on the same apartment;
                // `core` retains the interface and both UTF-16 literals are static.
                unsafe {
                    core.CallDevToolsProtocolMethod(
                        w!("Network.clearBrowserCache"),
                        w!("{}"),
                        &handler,
                    )
                }
            })();
            if let Err(error) = result {
                finish(&disk_finished, Err(error));
            }
            Ok(())
        }));
        // SAFETY: The profile is retained on its owning apartment, the bitmask
        // selects only HTTP disk cache, and WebView2 retains the completion handler.
        unsafe { profile.ClearBrowsingData(COREWEBVIEW2_BROWSING_DATA_KINDS_DISK_CACHE, &handler) }
    })();
    if let Err(error) = result {
        finish(&completion, Err(error));
    }
}

fn finish(completion: &Rc<RefCell<Option<super::Completion>>>, result: windows_core::Result<()>) {
    if let Some(sender) = completion.borrow_mut().take() {
        let _ = sender
            .send(result.map_err(|error| format!("Could not clear the browser cache: {error}")));
    }
}
