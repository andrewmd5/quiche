use crate::rainway::{launch_rainway};
use crate::ui::callback::run_async;
use quiche::etc::constants::BootstrapError;
use quiche::updater::{apply, download_with_callback, install, verify, ActiveUpdate, UpdateType};
use web_view::WebView;

pub fn verify_update<T: 'static>(webview: &mut WebView<'_, T>, update: &ActiveUpdate) {
    let verification_complete = "verificationComplete";
    let error_callback = "verificationFailed";
    let hash = update.get_hash();
    let temp_file = update.get_temp_name();
    run_async(
        webview,
        move || verify(hash, temp_file),
        verification_complete.to_string(),
        error_callback.to_string(),
    );
}

pub fn launch_and_close<T: 'static>(webview: &mut WebView<'_, T>) {
    launch_rainway();
    webview.terminate();
}

pub fn apply_update<T: 'static>(webview: &mut WebView<'_, T>, update: &ActiveUpdate) {
    let update_complete = "updateComplete";
    let error_callback = "updateFailed";
    let temp_file = update.get_temp_name();
    let version = update.get_version();
    let install_path = update.install_path.clone();
    let update_type = update.update_type.clone();
    run_async(
        webview,
        move || match update_type {
            UpdateType::Install => install(temp_file),
            _ => {
                apply(install_path, temp_file, version)
            }
        },
        update_complete.to_string(),
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
