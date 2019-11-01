use crate::io::disk::get_dir_files;
use std::env::var_os;
use std::io::{Error, ErrorKind};
use std::os::windows::process::CommandExt;
use std::path::PathBuf;
use std::process::Command;
use winapi::um::fileapi::DeleteFileW;

pub fn take_ownership_of_dir(dir: &PathBuf) -> bool {
    if dir.exists() && dir.is_dir() {
        let mut takeown_path = match var_os("WINDIR") {
            Some(val) => PathBuf::from(val), //add the Windows path
            None => return false,
        };
        takeown_path.push("System32");
        takeown_path.push("takeown.exe");
        let dir_path = format!("\"{}\"", dir.to_string_lossy().to_owned());
        match Command::new(&takeown_path)
            .args(&["/F", &dir_path, "/A", "/R", "/D Y"])
            .creation_flags(0x08000000)
            .output()
        {
            Ok(_o) => return true,
            Err(_e) => return false,
        };
    }

    false
}

pub fn grant_full_permissions(dir: &PathBuf) -> bool {
    let admin = "*S-1-5-32-544:F";
    let system = "*S-1-5-18:F";
    let service = "*S-1-5-19:F";

    if dir.exists() && dir.is_dir() {
        let mut icacls_path = match var_os("WINDIR") {
            Some(val) => PathBuf::from(val), //add the Windows path
            None => return false,
        };
        icacls_path.push("System32");
        icacls_path.push("icacls.exe");
        let dir_path = format!("{}", dir.to_string_lossy().to_owned());
        if let Ok(_o) = Command::new(&icacls_path)
            .args(&[&dir_path, "/t", "/c", "/q", "/grant", admin])
            .creation_flags(0x08000000)
            .output()
        {
            log::info!("Provided the Administrator group with full permissions over the install path.");
        }
        if let Ok(_o) = Command::new(&icacls_path)
            .args(&[&dir_path, "/t", "/c", "/q", "/grant", system])
            .creation_flags(0x08000000)
            .output()
        {
            log::info!("Provided the SYSTEM group with full permissions over the install path.");
        }

        if let Ok(_o) = Command::new(&icacls_path)
            .args(&[&dir_path, "/t", "/c", "/q", "/grant", service])
            .creation_flags(0x08000000)
            .output()
        {
            log::info!(
                "Provided the LocalService group with full permissions over the install path."
            );
        }
        return true;
    }

    false
}

pub fn unblock_file(file: PathBuf) {
    if file.exists() && file.is_file() {
        let mut os_string = file.into_os_string().into_string().unwrap();
        os_string.push_str(":Zone.Identifier");
        unsafe {
            let file_name: Vec<_> = os_string.encode_utf16().chain(Some(0)).collect();
            DeleteFileW(file_name.as_ptr());
        }
    }
}

pub fn unblock_path(path: &PathBuf) -> Result<(), Error> {
    if !path.exists() {
        return Err(Error::from(ErrorKind::NotFound));
    }
    if !path.is_dir() {
        return Err(Error::from(ErrorKind::InvalidInput));
    }
    let files = get_dir_files(&path)?;
    for file in files {
        let file_path = PathBuf::from(&file);
        unblock_file(file_path);
    }
    Ok(())
}
