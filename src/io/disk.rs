use fs_extra::dir::get_dir_content;
use std::ffi::OsStr;
use std::fs::{self, ReadDir};
use std::io::Error;
use std::path::{Path, PathBuf};

/// checks if a directory contains all the files in a vector
pub fn dir_contains_all_files(files: Vec<String>, input: &String) -> bool {
    let dir_files = match get_dir_files(&input) {
        Some(fs) => fs,
        None => return false,
    };
    for file in files {
        if !(&dir_files).into_iter().any(|v| v.clone() == file) {
            return false;
        }
    }
    true
}

/// returns a vector of all the files in a folder.
pub fn get_dir_files(input: &String) -> Option<Vec<String>> {
    let input_path = Path::new(input);
    if !input_path.exists() {
        return None;
    }
    let dir_content = match get_dir_content(input_path) {
        Ok(dc) => dc,
        Err(_e) => return None,
    };
    let dir_files: Vec<String> = (&dir_content.files)
        .into_iter()
        .map(|file| file.replace(input, ""))
        .collect();
    Some(dir_files)
}

/// a hacky way to get just the file name for root directory files
fn get_filename(path: &PathBuf) -> String {
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

/// Deletes all the files in a directory
/// Allows you to supply a vector of files you'd like to exclude for deletion
pub fn delete_dir_contents(
    read_dir_res: Result<ReadDir, Error>,
    ignored: Vec<String>,
) -> Result<(), Error> {
    match read_dir_res {
        Ok(dir) => {
            for entry in dir {
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
                };
            }
        }
        Err(e) => return Err(e),
    }
    Ok(())
}
