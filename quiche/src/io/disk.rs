use fs_extra::dir::get_dir_content;
use std::ffi::OsStr;
use std::fs::{self, ReadDir};
use std::io::{Error, ErrorKind};
use std::path::{Path, PathBuf};

/// checks if a directory contains all the files in a vector
pub fn dir_contains_all_files(dir_path: &Path, files: &Vec<String>) -> bool {
    match get_dir_files(dir_path) {
        Ok(dir_files) => files.iter().all(|file| dir_files.contains(file)),
        Err(_e) => false,
    }
}

pub fn get_total_files(input: &Path) -> Result<u64, Error> {
    get_dir_files(&input)
        .map(|files| files.len() as u64)
        .map_err(|e| e)
}

/// returns a vector of all the files in a folder.
pub fn get_dir_files(input: &Path) -> Result<Vec<String>, Error> {
    if !input.exists() {
        return Err(Error::new(
            ErrorKind::NotFound,
            format!("The given input path does not exist: {}", input.display()),
        ));
    }
    let dir_content = match get_dir_content(input) {
        Ok(f) => f,
        Err(e) => return Err(Error::new(ErrorKind::Interrupted, e.to_string())),
    };
    let dir_files: Vec<String> = (&dir_content.files)
        .into_iter()
        .map(|file| file.replace(input.to_str().unwrap(), ""))
        .collect();
    Ok(dir_files)
}

/// a hacky way to get just the file name for root directory files
pub fn get_filename(path: &Path) -> String {
    let ext = String::from(
        path.extension()
            .unwrap_or(OsStr::new(""))
            .to_str()
            .unwrap_or_default(),
    );
    let stem = String::from(
        path.file_stem()
            .unwrap_or(OsStr::new(""))
            .to_str()
            .unwrap_or_default(),
    );
    let dot = match ext.is_empty() {
        true => "",
        false => ".",
    };
    format!("{}{}{}", stem, dot, ext)
}

/// normalizes Windows paths so they don't fucking blow up
pub fn to_slash(buf: &Path) -> PathBuf {
    let is_dir = buf.is_dir();
    use std::path;
    let components = buf
        .components()
        .map(|c| match c {
            path::Component::RootDir => Some(""),
            path::Component::CurDir => Some("."),
            path::Component::ParentDir => Some(".."),
            path::Component::Prefix(ref p) => p.as_os_str().to_str(),
            path::Component::Normal(ref s) => s.to_str(),
        })
        .collect::<Option<Vec<_>>>();

    let mut raw = components
        .map(|v| {
            if v.len() == 1 && v[0].is_empty() {
                // Special case for '/'
                "/".to_string()
            } else {
                v.join("/")
            }
        })
        .unwrap_or_default();

    if raw.is_empty() {
        return PathBuf::new();
    }
    if is_dir && raw.ends_with("") {
        raw.push('/');
    }
    PathBuf::from(raw)
}

/// Deletes all the files in a directory
/// Allows you to supply a vector of files you'd like to exclude for deletion
pub fn delete_dir_contents(
    read_dir_res: Result<ReadDir, Error>,
    ignored: &Vec<String>,
) -> Result<(), Error> {
    for entry in read_dir_res? {
        if let Ok(entry) = entry {
            let path = entry.path();
            if path.is_dir() {
                fs::remove_dir_all(path)?;
            } else {
                if (&ignored)
                    .into_iter()
                    .any(|v| v.clone() == get_filename(&path))
                {
                    continue;
                }
                fs::remove_file(path)?;
            }
        }
    }
    Ok(())
}
