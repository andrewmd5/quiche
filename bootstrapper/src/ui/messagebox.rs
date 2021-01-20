use quiche::os::windows::is_run_as_admin;
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
    use winapi::shared::winerror::SUCCEEDED;
    use winapi::um::combaseapi::{CoInitializeEx, CoUninitialize};
    use winapi::um::objbase::{COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE};
    use winapi::um::shellapi::ShellExecuteW;
    use winapi::um::winuser::SW_SHOWNORMAL;

    let open: Vec<_> = "open".encode_utf16().chain(Some(0)).collect();
    let url: Vec<_> = url.encode_utf16().chain(Some(0)).collect();

    unsafe {
        let coinitializeex_result = CoInitializeEx(
            ptr::null_mut(),
            COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE,
        );
        let code = ShellExecuteW(
            ptr::null_mut(),
            open.as_ptr(),
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

pub fn run_as(file: &Vec<u16>) -> bool {
    use std::ptr;
    use winapi::shared::winerror::SUCCEEDED;
    use winapi::um::combaseapi::{CoInitializeEx, CoUninitialize};
    use winapi::um::objbase::{COINIT_APARTMENTTHREADED, COINIT_DISABLE_OLE1DDE};
    use winapi::um::shellapi::ShellExecuteW;
    use winapi::um::winuser::SW_SHOWNORMAL;
    let runas: Vec<_> = "runas".encode_utf16().chain(Some(0)).collect();
    unsafe {
        let coinitializeex_result = CoInitializeEx(
            ptr::null_mut(),
            COINIT_APARTMENTTHREADED | COINIT_DISABLE_OLE1DDE,
        );
        let code = ShellExecuteW(
            ptr::null_mut(),
            runas.as_ptr(),
            file.as_ptr(),
            ptr::null(),
            ptr::null(),
            SW_SHOWNORMAL,
        ) as usize as i32;
        if SUCCEEDED(coinitializeex_result) {
            CoUninitialize();
        }
        code > 32
    }
}

pub fn try_elevate() -> bool {
    use std::ptr;
    use winapi::um::libloaderapi::GetModuleFileNameW;
    unsafe {
        if is_run_as_admin() {
            return false;
        }
        let mut buf = Vec::with_capacity(255);
        let ret = GetModuleFileNameW(ptr::null_mut(), buf.as_mut_ptr(), 255) as usize;
        if ret != 0 {
            buf.set_len(ret);
            return run_as(&buf);
        }
        false
    }
}
