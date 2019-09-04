use crate::os::windows::{open_url};
use std::ffi::CString;
use winapi::um::winuser::MessageBoxA;
use winapi::um::winuser::{MB_ICONERROR, MB_OK};

/// Presents a MessageBox error to the user.
pub fn show_error(caption: &'static str, text: String) {
    let lp_caption = CString::new(caption).unwrap();
    let lp_text = CString::new(text).unwrap();
    unsafe {
        let _button_id = MessageBoxA(
            std::ptr::null_mut(),
            lp_text.as_ptr(),
            lp_caption.as_ptr(),
            MB_OK | MB_ICONERROR,
        );
    }
}
/// Presents a MessageBox and after it is closed opens a URL in the systems default browser.
pub fn show_error_with_url(caption: &'static str, text: String, url: &'static str) {
    show_error(caption, text);
    open_url(url);
}
