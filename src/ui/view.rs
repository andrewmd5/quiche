use crate::ui::callback::run_async;
use crate::updater::{download_with_callback, verify, ActiveUpdate};
use web_view::WebView;

pub fn verify_update<T: 'static>(webview: &mut WebView<'_, T>, update: &ActiveUpdate) {
    let verification_complete = "verificationComplete";
    let error_callback = "verificationFailed";
    let hash = update.get_hash();
    let version = update.get_version();
    run_async(
        webview,
        move || verify(hash, version),
        verification_complete.to_string(),
        error_callback.to_string(),
    );
}

pub fn download_update<T: 'static>(webview: &mut WebView<'_, T>, update: &ActiveUpdate) {
    let url = update.get_url();
    let version = update.get_version();
    let temp_file = update.get_temp_name();
    let download_complete = "downloadComplete";
    let error_callback = "downloadFailed";
    let handle = webview.handle();
    run_async(
        webview,
        move || {
            let func_test = move |total_bytes: u64, downloaded_bytes: u64| {
                let data = format!(
                    "downloadProgress('{}', '{}', '{}')",
                    version, total_bytes, downloaded_bytes
                );
                handle
                    .dispatch(move |webview| webview.eval(&data.to_string()))
                    .unwrap();
            };
            download_with_callback(url, temp_file, func_test)
        },
        download_complete.to_string(),
        error_callback.to_string(),
    );
}
