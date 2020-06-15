use winapi::shared::minwindef::{DWORD, FALSE, HIBYTE, LOBYTE, WORD};
//
// _WIN32_WINNT version constants
//
const _WIN32_WINNT_NT4: WORD = 0x0400;
const _WIN32_WINNT_WIN2K: WORD = 0x0500;
const _WIN32_WINNT_WINXP: WORD = 0x0501;
const _WIN32_WINNT_WS03: WORD = 0x0502;
const _WIN32_WINNT_WIN6: WORD = 0x0600;
const _WIN32_WINNT_VISTA: WORD = 0x0600;
const _WIN32_WINNT_WS08: WORD = 0x0600;
const _WIN32_WINNT_LONGHORN: WORD = 0x0600;
const _WIN32_WINNT_WIN7: WORD = 0x0601;
const _WIN32_WINNT_WIN8: WORD = 0x0602;
const _WIN32_WINNT_WINBLUE: WORD = 0x0603;
const _WIN32_WINNT_WINTHRESHOLD: WORD = 0x0A00;
const _WIN32_WINNT_WIN10: WORD = 0x0A00;
use winapi::um::sysinfoapi::VerSetConditionMask;
use winapi::um::winbase::VerifyVersionInfoW;
use winapi::um::winnt::{
    DWORDLONG, OSVERSIONINFOEXW, VER_EQUAL, VER_GREATER_EQUAL, VER_MAJORVERSION, VER_MINORVERSION,
    VER_NT_WORKSTATION, VER_PRODUCT_TYPE, VER_SERVICEPACKMAJOR,
};

/// Indicates if the current OS version matches,
/// or is greater than, the provided version information.
/// This function is useful in confirming a version of Windows Server
/// that doesn't share a version number with a client release.
pub fn is_windows_version_or_greater(
    major_version: WORD,
    minor_version: WORD,
    service_pack_major: WORD,
) -> bool {
    use core::mem::{size_of};
    unsafe {
        let condition_mask: DWORDLONG = VerSetConditionMask(
            VerSetConditionMask(
                VerSetConditionMask(0 as DWORDLONG, VER_MAJORVERSION, VER_GREATER_EQUAL),
                VER_MINORVERSION,
                VER_GREATER_EQUAL,
            ),
            VER_SERVICEPACKMAJOR,
            VER_GREATER_EQUAL,
        );
        let mut osvi: OSVERSIONINFOEXW = std::mem::MaybeUninit::zeroed().assume_init();
        osvi.dwOSVersionInfoSize = size_of::<OSVERSIONINFOEXW>() as DWORD;
        osvi.dwMajorVersion = major_version as DWORD;
        osvi.dwMinorVersion = minor_version as DWORD;
        osvi.wServicePackMajor = service_pack_major;
        VerifyVersionInfoW(
            &mut osvi,
            VER_MAJORVERSION | VER_MINORVERSION | VER_SERVICEPACKMAJOR,
            condition_mask,
        ) != FALSE
    }
}


pub fn is_windows_xpor_greater() -> bool {
    is_windows_version_or_greater(
        HIBYTE(_WIN32_WINNT_WINXP) as WORD,
        LOBYTE(_WIN32_WINNT_WINXP) as WORD,
        0 as WORD,
    )
}

pub fn is_windows_xpsp1_or_greater() -> bool {
    is_windows_version_or_greater(
        HIBYTE(_WIN32_WINNT_WINXP) as WORD,
        LOBYTE(_WIN32_WINNT_WINXP) as WORD,
        1 as WORD,
    )
}

pub fn is_windows_xpsp2_or_greater() -> bool {
    is_windows_version_or_greater(
        HIBYTE(_WIN32_WINNT_WINXP) as WORD,
        LOBYTE(_WIN32_WINNT_WINXP) as WORD,
        2 as WORD,
    )
}

pub fn is_windows_xpsp3_or_greater() -> bool {
    is_windows_version_or_greater(
        HIBYTE(_WIN32_WINNT_WINXP) as WORD,
        LOBYTE(_WIN32_WINNT_WINXP) as WORD,
        3 as WORD,
    )
}

pub fn is_windows_vista_or_greater() -> bool {
    is_windows_version_or_greater(
        HIBYTE(_WIN32_WINNT_VISTA) as WORD,
        LOBYTE(_WIN32_WINNT_VISTA) as WORD,
        0 as WORD,
    )
}

pub fn is_windows_vista_sp1_or_greater() -> bool {
    is_windows_version_or_greater(
        HIBYTE(_WIN32_WINNT_VISTA) as WORD,
        LOBYTE(_WIN32_WINNT_VISTA) as WORD,
        1 as WORD,
    )
}

pub fn is_windows_vista_sp2_or_greater() -> bool {
    is_windows_version_or_greater(
        HIBYTE(_WIN32_WINNT_VISTA) as WORD,
        LOBYTE(_WIN32_WINNT_VISTA) as WORD,
        2 as WORD,
    )
}

pub fn is_windows7_or_greater() -> bool {
    is_windows_version_or_greater(
        HIBYTE(_WIN32_WINNT_WIN7) as WORD,
        LOBYTE(_WIN32_WINNT_WIN7) as WORD,
        0 as WORD,
    )
}

pub fn is_windows7_sp1_or_greater() -> bool {
    is_windows_version_or_greater(
        HIBYTE(_WIN32_WINNT_WIN7) as WORD,
        LOBYTE(_WIN32_WINNT_WIN7) as WORD,
        1 as WORD,
    )
}

pub fn is_windows8_or_greater() -> bool {
    is_windows_version_or_greater(
        HIBYTE(_WIN32_WINNT_WIN8) as WORD,
        LOBYTE(_WIN32_WINNT_WIN8) as WORD,
        0 as WORD,
    )
}

pub fn is_windows8_point1_or_greater() -> bool {
    is_windows_version_or_greater(
        HIBYTE(_WIN32_WINNT_WINBLUE) as WORD,
        LOBYTE(_WIN32_WINNT_WINBLUE) as WORD,
        0 as WORD,
    )
}

/// The same as calling is_windows10_or_greater
pub fn is_windows_threshold_or_greater() -> bool {
    is_windows_version_or_greater(
        HIBYTE(_WIN32_WINNT_WINTHRESHOLD) as WORD,
        LOBYTE(_WIN32_WINNT_WINTHRESHOLD) as WORD,
        0 as WORD,
    )
}
/// Indicates if the current OS version matches,
/// or is greater than, the Windows 10 version.
/// For Windows 10, IsWindows10OrGreater returns false unless the application contains
/// a manifest that includes a compatibility section that contains the GUID that designates Windows 10.
pub fn is_windows10_or_greater() -> bool {
    is_windows_version_or_greater(
        HIBYTE(_WIN32_WINNT_WIN10) as WORD,
        LOBYTE(_WIN32_WINNT_WIN10) as WORD,
        0 as WORD,
    )
}

/// Indicates if the current OS is a Windows Server release.
/// Applications that need to distinguish between server and client versions
/// of Windows should call this function.
pub fn is_windows_server() -> bool {
    use core::mem::{size_of};
    unsafe {
        let condition_mask = VerSetConditionMask(0 as DWORDLONG, VER_PRODUCT_TYPE, VER_EQUAL);
        let mut osvi: OSVERSIONINFOEXW = std::mem::MaybeUninit::zeroed().assume_init();
        osvi.dwOSVersionInfoSize = size_of::<OSVERSIONINFOEXW>() as DWORD;
        osvi.wProductType = VER_NT_WORKSTATION;
        VerifyVersionInfoW(&mut osvi, VER_PRODUCT_TYPE, condition_mask) == FALSE
    }
}
