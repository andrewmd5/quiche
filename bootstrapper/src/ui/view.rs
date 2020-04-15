use crate::rainway::launch_rainway;
use crate::ui::callback::run_async;

use quiche::updater::{apply, download_with_callback, install, verify, ActiveUpdate, UpdateType};
use web_view::WebView;

pub fn verify_update<T: 'static>(webview: &mut WebView<'_, T>, update: &ActiveUpdate) {
    let verification_complete = "verificationComplete";
    let error_callback = "verificationFailed";
    let ud = update.clone();
    run_async(
        webview,
        move || verify(ud),
        verification_complete.to_string(),
        error_callback.to_string(),
    );
}

pub fn launch_and_close<T: 'static>(_webview: &mut WebView<'_, T>, update: &ActiveUpdate) {
    launch_rainway(&update.install_info.path);
    std::process::exit(0);
}

pub fn apply_update<T: 'static>(webview: &mut WebView<'_, T>, update: &ActiveUpdate) {
    let update_complete = "updateComplete";
    let error_callback = "updateFailed";
    let mut ud = update.clone();
    run_async(
        webview,
        move || match ud.update_type {
            UpdateType::Install => install(&mut ud),
            UpdateType::Patch => apply(ud),
        },
        update_complete.to_string(),
        error_callback.to_string(),
    );
}

pub fn download_update<T: 'static>(webview: &mut WebView<'_, T>, update: &ActiveUpdate) {
    let download_complete = "downloadComplete";
    let error_callback = "downloadFailed";
    let handle = webview.handle();
    let ud = update.clone();
    run_async(
        webview,
        move || {
            let version = ud.get_version();
            let download_progress = move |total_bytes: u64, downloaded_bytes: u64| {
                let data = format!(
                    "downloadProgress('{}', '{}', '{}')",
                    version, total_bytes, downloaded_bytes
                );
                handle
                    .dispatch(move |webview| webview.eval(&data.to_string()))
                    .unwrap();
            };
            download_with_callback(ud, download_progress)
        },
        download_complete.to_string(),
        error_callback.to_string(),
    );
}
