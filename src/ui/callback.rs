use std::thread;
use web_view::WebView;

/// runs a task on a new thread as not to block the UI
pub fn run_async<T: 'static, F: FnOnce() -> Result<String, String> + Send + 'static>(
    webview: &mut WebView<'_, T>,
    what: F,
    callback: String,
    error: String,
) {
    let handle = webview.handle();
    thread::spawn(move || {
        let callback_string = format_callback_result(what(), callback, error);
        handle
            .dispatch(move |_webview| _webview.eval(callback_string.as_str()))
            .unwrap()
    });
}

/// Escape a string to pass it into JavaScript.
fn escape_string(arg: String) -> String {
    let mut escaped_string = String::default();
    escaped_string.push('\'');
    for c in arg.chars() {
        match c {
            '"' => escaped_string.push_str("\\\""),
            '\\' => escaped_string.push_str("\\\\"),
            '\n' => escaped_string.push_str("\\n"),
            '\r' => escaped_string.push_str("\\r"),
            '\'' => escaped_string.push_str("\\'"),
            '\t' => escaped_string.push_str("\\t"),
            _ => {
                let i = c as i32;
                if i < 32 || i > 127 {
                    escaped_string.push_str(format!("\\u{:04x}", i).as_str());
                } else {
                    escaped_string.push_str(c.to_string().as_str());
                }
            }
        }
    }
    escaped_string.push('\'');
    return escaped_string;
}

/// Formats a callback in to a javascript function call
fn format_callback(function_name: String, arg: String) -> String {
    let escaped_arg = escape_string(arg);
    let formatted_string = &format!("{}({})", function_name, escaped_arg);
    return formatted_string.to_string();
}

fn format_callback_result(
    result: Result<String, String>,
    callback: String,
    error_callback: String,
) -> String {
    match result {
        Ok(res) => return format_callback(callback, res),
        Err(err) => return format_callback(error_callback, err),
    }
}
