use threadpool::ThreadPool;
use web_view::*;

thread_local!(static POOL: ThreadPool = ThreadPool::new(4));

pub fn run_async<T: 'static, F: FnOnce() -> Result<String, String> + Send + 'static>(
    webview: &mut WebView<'_, T>,
    what: F,
    callback: String,
    error: String,
) {
    let handle = webview.handle();
    POOL.with(|thread| {
        thread.execute(move || {
            let callback_string = format_callback_result(what(), callback, error);
            handle
                .dispatch(move |_webview| _webview.eval(callback_string.as_str()))
                .unwrap()
        });
    });
}

fn format_callback(function_name: String, arg: String) -> String {
    let formatted_string = &format!("{}({})", function_name, arg);
    return formatted_string.to_string();
}

fn format_callback_result(
    result: Result<String, String>,
    callback: String,
    error_callback: String,
) -> String {
    match result {
        Ok(res) => return format_callback(callback, res),
        Err(err) => return format_callback(error_callback, format!("\"{}\"", err)),
    }
}
