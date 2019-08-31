use winapi::shared::windef::{RECT};
use winapi::um::wincon::GetConsoleWindow;
use winapi::um::winuser::{
    GetDesktopWindow, GetWindowRect, MoveWindow, SetWindowPos, HWND_NOTOPMOST,
    SWP_SHOWWINDOW,
};

pub fn center_window() {
    unsafe {
        let mut desktop_rect: RECT = RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        GetWindowRect(GetDesktopWindow(), &mut desktop_rect);
        SetWindowPos(
            GetConsoleWindow(),
            HWND_NOTOPMOST,
            0,
            0,
            1020,
            358,
            SWP_SHOWWINDOW,
        );
        let mut console_rect: RECT = RECT {
            left: 0,
            top: 0,
            right: 0,
            bottom: 0,
        };
        GetWindowRect(GetConsoleWindow(), &mut console_rect);

        let width = console_rect.right - console_rect.left;
        let height = console_rect.bottom - console_rect.top;

        let console_x = (desktop_rect.right - desktop_rect.left) / 2 - width / 2;
        let console_y = (desktop_rect.bottom - desktop_rect.top) / 2 - height / 2;

        //SetWindowPos(GetConsoleWindow(),NULL,ConsolePosX,ConsolePosY, Width, Height, SWP_SHOWWINDOW || SWP_NOSIZE);

        MoveWindow(GetConsoleWindow(), console_x, console_y, width, height, 1);
        println!(
            "({}, {}), ({}, {}) - {}x{}",
            desktop_rect.left,
            desktop_rect.top,
            desktop_rect.right,
            desktop_rect.bottom,
            desktop_rect.right - desktop_rect.left,
            desktop_rect.bottom - desktop_rect.top
        );
    }
}
