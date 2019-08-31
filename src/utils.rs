use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::io::Seek;
use std::path::PathBuf;
use std::ptr;
use std::{fs, io};
use widestring::U16CString;

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
/// Release info is pulled from a remote JSON config [here](https://releases.rainway.com/Installer_current.json).
/// The information located inside that config can be used to form a download URL.
pub struct ReleaseInfo {
    /// The prefix on our installer.
    pub name: String,
    /// The current release version.
    pub version: String,
    /// The SHA256 hash of the installer.
    /// Used to validate if the file downloaded properly.
    pub hash: String,
}

/// hashes a file using SHA256 and returns the formatted `{:X}` String.
pub fn hash_file(path: &PathBuf) -> Option<String> {
    if let Ok(mut file) = fs::File::open(path) {
        &file.seek(std::io::SeekFrom::Start(0));
        let mut sha256 = Sha256::new();
        io::copy(&mut file, &mut sha256).unwrap_or(0);
        return Some(format!("{:X}", sha256.result()));
    };
    None
}
/// checks if the executable has been compiled against a x64 target.
pub fn is_compiled_for_64_bit() -> bool {
    cfg!(target_pointer_width = "64")
}

pub fn open_url(url: &'static str) {
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
