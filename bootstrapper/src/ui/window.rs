/// makes the applications WebView DPI aware.
pub fn set_dpi_aware() {
    use winapi::um::shellscalingapi::{SetProcessDpiAwareness, PROCESS_SYSTEM_DPI_AWARE};
    unsafe { SetProcessDpiAwareness(PROCESS_SYSTEM_DPI_AWARE) };
}

/*pub fn center_window() {
    unsafe {

        let window = GetActiveWindow();
        let mut desktop_rect: RECT = RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        GetWindowRect(GetDesktopWindow(), &mut desktop_rect);
        SetWindowPos(
            window,
            HWND_NOTOPMOST,
            0,
            0,
            600,
            380,
            SWP_SHOWWINDOW,
        );
        let mut console_rect: RECT = RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        GetWindowRect(window, &mut console_rect);

        let width = console_rect.right - console_rect.left;
        let height = console_rect.bottom - console_rect.top;

        let console_x = (desktop_rect.right - desktop_rect.left) / 2 - width / 2;
        let console_y = (desktop_rect.bottom - desktop_rect.top) / 2 - height / 2;

        MoveWindow(window, console_x, console_y, width, height, 1);
    }
}*/
