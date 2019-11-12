use crate::os::process::get_procs_using_path;
use std::fs::{self, copy, create_dir_all, remove_file, rename};
use std::io::{Error, ErrorKind};
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

/// checks if a directory contains all the files in a vector
pub fn dir_contains_all_files(dir_path: &Path, files: &Vec<String>) -> bool {
    match get_dir_files(dir_path) {
        Ok(dir_files) => files.iter().all(|file| dir_files.contains(file)),
        Err(_e) => false,
    }
}
/// counts all the files in a directory
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
    let mut dir_files: Vec<String> = Vec::new();
    for entry in WalkDir::new(&input) {
        let entry = entry?;
        if entry.path().is_dir() {
            continue;
        }
        if !entry.path().exists() {
            return Err(Error::from(ErrorKind::NotFound));
        }
        match entry.path().strip_prefix(input) {
            Ok(p) => {
                if p.to_str().is_none() {
                    return Err(Error::from(ErrorKind::InvalidInput));
                }
                dir_files.push(p.to_str().unwrap().to_string());
            }
            Err(_e) => return Err(Error::from(ErrorKind::InvalidInput)),
        }
    }
    Ok(dir_files)
}

/// safely unwrap the paths file name if it exist
pub fn get_filename(path: &Path) -> String {
    if let Some(os_name) = path.file_name() {
        return os_name.to_string_lossy().to_string();
    }
    path.to_string_lossy().to_owned().to_string()
}

/// without normalizing the paths to files/directories, Rust will be unable
/// to deep copy nested directories properly.
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
                "\\".to_string()
            } else {
                v.join("\\")
            }
        })
        .unwrap_or_default();

    if raw.is_empty() {
        return PathBuf::new();
    }
    if is_dir && raw.ends_with("") {
        raw.push('\\');
    }
    raw = raw.replace("\\\\", "\\");
    PathBuf::from(raw)
}

/// Deletes all the files in a directory
/// Allows you to supply a vector of files you'd like to exclude for deletion
pub fn delete_dir_contents(path: &Path, ignored: &Vec<String>) -> Result<(), Error> {
    for entry in WalkDir::new(&path).into_iter().filter_map(|e| e.ok()) {
        if entry.path().exists() && entry.path().is_file() {
            if (&ignored)
                .into_iter()
                .any(|v| v.clone() == get_filename(&entry.path()))
            {
                continue;
            }
            fs::remove_file(entry.path())?;
        }
    }
    for entry in WalkDir::new(&path).into_iter().filter_map(|e| e.ok()) {
        if entry.path().exists() && entry.path().is_dir() {
            let is_empty = entry
                .path()
                .read_dir()
                .map(|mut i| i.next().is_none())
                .unwrap_or(false);
            if is_empty {
                fs::remove_dir_all(entry.path())?;
            }
        }
    }
    Ok(())
}

/// moves a file
pub fn move_file(from: &Path, to: &Path) -> Result<(), Error> {
    link_file(&from, &to, false)
}

/// copies a file
pub fn copy_file(from: &Path, to: &Path) -> Result<(), Error> {
    link_file(&from, &to, true)
}

/// attempts to move or copy a file to a destination path.
/// this will return an error if the file is locked, or otherwise not available.
fn link_file(from: &Path, to: &Path, copy_file: bool) -> Result<(), Error> {
    if !from.exists() {
        log::error!("The source file path does not exist {}", &from.display());
        return Err(Error::from(ErrorKind::NotFound));
    }
    if !from.is_file() {
        log::error!(
            "The source file path is not a executable {}",
            &from.display()
        );
        return Err(Error::from(ErrorKind::InvalidInput));
    }
    if to.exists() {
        remove_file(&to)?;
    }
    let lockers = get_procs_using_path(&from)?;
    if lockers.len() > 0 {
        for lock in lockers {
            log::info!("{} is locking {}", lock.name(), &from.display());
            lock.kill();
            if !lock.is_running() {
                log::info!("{} has been terminated.", lock.name());
            } else {
                log::error!(
                    "unable to terminate {} so we're stopping here.",
                    lock.name()
                );
                return Err(Error::from(ErrorKind::Interrupted));
            }
        }
    }
    // test if we can access the file. there could be a driver level lock on it.
    if let Err(e) = rename(&from, &from) {
        log::error!(
            "unable to access {} even after clearing locks.",
            &from.display()
        );
        return Err(e);
    }
    if copy_file {
        copy(&from, &to)?;
    } else {
        // we need to copy first then remove just incase the files are on different disk.
        copy(&from, &to)?;
        remove_file(&from)?;
    }
    Ok(())
}

/// copies the contents of a directory
pub fn copy_dir(from: &Path, to: &Path, ignored: &Vec<String>) -> Result<(), Error> {
    move_dir_contents(&from, &to, &ignored, true)
}
/// moves the contents of a directory
pub fn move_dir(from: &Path, to: &Path, ignored: &Vec<String>) -> Result<(), Error> {
    move_dir_contents(&from, &to, &ignored, false)
}
/// relocates the full contents (files/directories) of a given path.
/// quite a number of checks are done to ensure the operation is safely handled.
fn move_dir_contents(
    from: &Path,
    to: &Path,
    ignored: &Vec<String>,
    copy_contents: bool,
) -> Result<(), Error> {
    if !from.exists() {
        log::error!("The source dir path does not exist {}", &from.display());
        return Err(Error::from(ErrorKind::NotFound));
    }
    if !from.is_dir() {
        log::error!("The source dir path is not a directory {}", &from.display());
        return Err(Error::from(ErrorKind::InvalidInput));
    }
    if !to.exists() {
        create_dir_all(&to)?;
        log::info!("created {}", &to.display());
    } else {
        delete_dir_contents(&to, &ignored)?;
        log::info!("deleted contents of {}", &to.display());
    }
    // lets build our directory structures first
    for entry in WalkDir::new(&from).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.exists() && path.is_dir() {
            let tmp_to = match Path::new(&path).strip_prefix(&from) {
                Ok(p) => p,
                Err(_e) => return Err(Error::from(ErrorKind::InvalidInput)),
            };
            let dir = &to.join(&tmp_to);
            create_dir_all(&dir)?;
        }
    }
    // now we complete the file tree
    for entry in WalkDir::new(&from).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.exists() && path.is_file() {
            if (&ignored)
                .into_iter()
                .any(|v| v.clone() == get_filename(&path))
            {
                log::warn!("{}", &path.display());
                continue;
            }

            let tmp_to = match Path::new(&path).strip_prefix(&from) {
                Ok(p) => p,
                Err(_e) => return Err(Error::from(ErrorKind::InvalidInput)),
            };
            let file = &to.join(&tmp_to);
            if copy_contents {
                copy_file(path, &file)?;
            } else {
                move_file(&path, &file)?;
            }
        }
    }
    // then for good measure, lets clean up the original directory.
    if !copy_contents {
        delete_dir_contents(&from, &ignored)?;
        log::info!("deleted the remaining contents of {}", &from.display());
    }
    Ok(())
}

pub fn swap_files(from: &Path, to: &Path) -> Result<(), Error> {
    if !from.exists() {
        log::error!("The source file does not exist {}", &from.display());
        return Err(Error::from(ErrorKind::NotFound));
    }
    if !from.is_file() {
        log::error!("The source path is not a file {}", &from.display());
        return Err(Error::from(ErrorKind::InvalidInput));
    }
    if !to.exists() {
        log::error!("The output file does not exist {}", &from.display());
        return Err(Error::from(ErrorKind::NotFound));
    }
    if !to.is_file() {
        log::error!("The output path is not a file {}", &from.display());
        return Err(Error::from(ErrorKind::InvalidInput));
    }
    if let Some(parent) = from.parent() {
        let current_file = get_filename(&from);
        let old_file = PathBuf::from(format!("{}\\{}_old", parent.display(), current_file));
        rename(&from, &old_file)?;
        match rename(&to, &from) {
            Err(_e) => rename(&old_file, &from)?,
            Ok(f) => f,
        }
        Ok(())
    } else {
        log::error!(
            "Cannot locate parent directory of {} for file swap",
            &from.display()
        );
        return Err(Error::from(ErrorKind::InvalidInput));
    }
}
