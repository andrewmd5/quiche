use crate::ui::callback::run_async;
use crate::updater::ActiveUpdate;
use web_view::WebView;
use crate::updater::download_callback;

pub fn download<T: 'static>(webview: &mut WebView<'_, T>, update: &ActiveUpdate) {
    let url = update.get_url();
    let version = update.get_version();
    let download_complete = "downloadComplete";
    let error_callback = "downloadFailed";
    let handle = webview.handle();
    run_async(
        webview,
        move || {
            let func_test = move |version: String, total_bytes: u64, downloaded_bytes: u64| {
                let data = format!(
                    "downloadProgress('{}', '{}', '{}')",
                    version, total_bytes, downloaded_bytes
                );
                handle
                    .dispatch(move |webview| webview.eval(&data.to_string()))
                    .unwrap();
            };
            download_callback(url, version, func_test)
        },
        download_complete.to_string(),
        error_callback.to_string(),
    );
}