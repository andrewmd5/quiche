use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::io::Seek;
use std::path::PathBuf;
use std::{fs, io};

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
