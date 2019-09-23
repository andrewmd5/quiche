use sha2::{Digest, Sha256};
use std::io::Seek;
use std::path::PathBuf;
use std::{fs, io};

/// hashes a file using SHA256 and returns the formatted `{:X}` String.
pub fn sha_256(path: &PathBuf) -> Option<String> {
    if let Ok(mut file) = fs::File::open(path) {
        &file.seek(std::io::SeekFrom::Start(0));
        let mut sha256 = Sha256::new();
        io::copy(&mut file, &mut sha256).unwrap_or(0);
        return Some(format!("{:X}", sha256.result()));
    };
    None
}
