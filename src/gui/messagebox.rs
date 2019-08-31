use crate::utils;
use std::ffi::CString;
use user32::MessageBoxA;
use winapi::um::winuser::{MB_ICONERROR, MB_OK};

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

pub fn show_error_with_url(caption: &'static str, text: String, url: &'static str) {
    show_error(caption, text);
    utils::open_url(url);
}
