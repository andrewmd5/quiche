use crate::callback;
use crate::httpclient::{download_json, download_toml};
use crate::rainway::get_version;
use crate::utils::hash_file;
use crate::utils::ReleaseInfo;
use serde::Deserialize;
use std::env;
use std::fs::{self, DirEntry};
use std::io;
use std::path::Path;
use web_view::WebView;
use web_view::*;

#[derive(Deserialize)]
/// Holds information about various Rainway releases
struct Releases {
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
pub struct Update {
    /// Not used by the updater, however it allows us to track changes.
    pub fresh_files: Vec<String>,
    /// A list of all the files deleted as compared to the last version.
    /// Allows the bootstrapper to delete them.
    pub deleted_files: Vec<String>,
    /// The update package.
    pub package: Package,
    /// The full installer.
    pub installer: Installer,
}

#[derive(Deserialize)]
pub struct Installer {
    /// The URL of the actual full installer.
    pub url: String,
    /// The hash of the installer to verify it downloaded properly.
    pub hash: String,
}

#[derive(Deserialize)]
pub struct Package {
    /// The URL to the zip package containing all the new files.
    pub zip: String,
    /// A hash of the zip package, used to verify it downloaded properly.
    pub hash: String,
}

#[derive(Debug, Clone, Default)]
struct FileInfo {
    pub full_path: String,
    pub path: String,
    pub hash: String,
}

//Checks for and returns any required updates the current installation needs
pub fn check_for_updates() -> Option<Vec<Update>> {
    let installed_version = get_version(); //TODO check if pulled. If issue, return false.
                                       //TODO check if downloaded/parsed. If issue, return false.
    let mut releases =
        download_toml::<Releases>("http://192.168.153.1:8080/Releases.toml").unwrap();
    releases.versions.past.reverse();
    if installed_version == releases.versions.current {
        println!("Rainway is up-to-date.");
        return None;
    }
    let latest_manifest_url = format!(
        "http://192.168.153.1:8080/{}/manifest.toml",
        releases.versions.current
    );
    //TODO check if we could download the latest manifest
    let latest_manifest = download_toml::<Update>(&latest_manifest_url).unwrap();

    //TODO safely check if the currently installed version is the last good version
    //then return it since we only need _this_ update
    if releases.versions.past.last().unwrap() == &installed_version {
        return Some(vec![latest_manifest]);
    }
    let mut updates: Vec<Update> = Vec::new();
    // TODO check if this is a valid version safely
    let installed_version_index = releases
        .versions
        .past
        .iter()
        .position(|r| r == &installed_version)
        .unwrap();

    //TODO get all inbetween updates safely
    for i in installed_version_index + 1..releases.versions.past.len() {
        let version_manifest_url = format!(
            "http://192.168.153.1:8080/{}/manifest.toml",
            releases.versions.past[i]
        );
        let version_manifest = download_toml::<Update>(&version_manifest_url).unwrap();
        updates.push(version_manifest);
    }
    updates.push(latest_manifest);
    Some(updates)
}

//gets the latest Rainway release. Used for installing Rainway.
pub fn get_latest_release() -> Option<Update> {
    let mut releases =
        download_toml::<Releases>("http://192.168.153.1:8080/Releases.toml").unwrap();
    let latest_manifest_url = format!(
        "http://192.168.153.1:8080/{}/manifest.toml",
        releases.versions.current
    );
    //TODO check if we could download the latest manifest
    let latest_manifest = download_toml::<Update>(&latest_manifest_url).unwrap();
    Some(latest_manifest)
}

pub fn download_package() {}

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
