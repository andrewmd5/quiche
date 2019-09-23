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

/// opens a URL in the systems default web browser.
pub fn open_url(url: &'static str) {
    use std::ptr;
    use widestring::U16CString;
    use winapi::shared::winerror::SUCCEEDED;
    use winapi::um::combaseapi::{CoInitializeEx, CoUninitialize};
    use winapi::um::objbase::{COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE};
    use winapi::um::shellapi::ShellExecuteW;
    use winapi::um::winuser::SW_SHOWNORMAL;

    static OPEN: &[u16] = &['o' as u16, 'p' as u16, 'e' as u16, 'n' as u16, 0x0000];
    let url = U16CString::from_str(url).unwrap();
    unsafe {
        let coinitializeex_result = CoInitializeEx(
            ptr::null_mut(),
            COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE,
        );
        let code = ShellExecuteW(
            ptr::null_mut(),
            OPEN.as_ptr(),
            url.as_ptr(),
            ptr::null(),
            ptr::null(),
            SW_SHOWNORMAL,
        ) as usize as i32;
        if SUCCEEDED(coinitializeex_result) {
            CoUninitialize();
        }
        code
    };
}
