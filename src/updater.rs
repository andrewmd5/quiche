use crate::httpclient::download_toml;
use crate::utils::hash_file;
use serde::Deserialize;
use std::fs::{self, DirEntry};
use std::io;
use std::path::Path;

#[derive(Deserialize)]
/// Holds information about various Rainway releases
struct Releases {
    /// A URL string that can be formatted to get the manifest URL for a version.
    base_manifest_url: String,
    versions: Versions,
}

#[derive(Deserialize)]
/// A structure that presents various versions of Rainway
struct Versions {
    /// The current version of Rainway
    current: String,
    /// All past versions of Rainway, listed in descending order.
    /// This will allow any version of Rainway to upgrade to the latest.
    past: Vec<String>,
}

#[derive(Deserialize)]
struct Update {
    /// Not used by the updater, however it allows us to track changes.
    fresh_files: Vec<String>,
    /// A list of all the files deleted as compared to the last version.
    /// Allows the bootstrapper to delete them.
    deleted_files: Vec<String>,
    /// The update package.
    package: Package,
    /// The full installer.
    installer: Installer,
}

#[derive(Deserialize)]
struct Installer {
    /// The URL of the actual full installer.
    url: String,
    /// The hash of the installer to verify it downloaded properly.
    hash: String,
}

#[derive(Deserialize)]
struct Package {
    /// The URL to the zip package containing all the new files.
    zip: String,
    /// A hash of the zip package, used to verify it downloaded properly.
    hash: String,
}

#[derive(Debug, Clone, Default)]
struct FileInfo {
    pub full_path: String,
    pub path: String,
    pub hash: String,
}

pub fn get_releases() {
    let my_version = "1.0.2";
    let mut releases = download_toml::<Releases>("http://192.168.153.1:8080/Releases.toml").unwrap();
    releases.versions.past.reverse();
    if my_version == releases.versions.current {
        println!("Rainway up-to-date");
    } else {

        let installed_version_index = releases
            .versions
            .past
            .iter()
            .position(|r| r == my_version)
            .unwrap();

        for i in installed_version_index+1..releases.versions.past.len()  {
            println!("Need to update to version {} ", releases.versions.past[i]);
        }

       
    }
}

pub fn create_snapshot_manifest(previous_version: &str, new_version: &str) {
    let old_path = Path::new(previous_version);
    let new_path = Path::new(new_version);

    let old_files = visit_dirs(old_path, previous_version).unwrap();
    let new_files = visit_dirs(new_path, new_version).unwrap();

    let deleted_files: Vec<FileInfo> = old_files
        .into_iter()
        .filter(|x| !new_files.clone().into_iter().any(|u| u.path == x.path))
        .collect();

    for df in deleted_files {
        println!("{} {}", df.path, df.full_path);
    }
}

fn visit_dirs(dir: &Path, version: &str) -> Option<Vec<FileInfo>> {
    if dir.is_dir() {
        let mut files: Vec<FileInfo> = Vec::new();
        for entry in fs::read_dir(dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_dir() {
                if let Some(mut f) = visit_dirs(&path, version) {
                    files.append(&mut f);
                }
            } else {
                let full = path.as_path().to_str().unwrap();
                files.push(FileInfo {
                    full_path: full.to_string(),
                    path: full.to_string().replace(version, ""),
                    hash: hash_file(&path).unwrap(),
                });
            }
        }
        return Some(files);
    }
    None
}
