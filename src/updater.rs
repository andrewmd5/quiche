use crate::io::hash::sha_256;
use crate::net::http::{download_json, download_toml};
use serde::Deserialize;
use std::fs;
use std::path::Path;


#[derive(Debug)]
pub enum UpdateType {
    /// An install requires us to run the full Rainway installer, 
    /// becasuse Rainway itself is not installed.
    Install,
    /// A patch is applying new files to an existing installation.
    /// You have a hammer. After five months, you replace the head. 
    /// After five more months, you replace the handle. 
    /// Is it still the same hammer?
    Patch
}

#[derive(Debug)]
pub enum ReleaseBranch {
    Stable,
    Beta,
    Nightly,
}

/// The various states an update can be in.
#[derive(Debug, PartialEq, Eq)]
pub enum UpdateState {
    /// None means the update/installation has not started.
    None,
    /// Failed signifies that the update failed,
    /// usually because the download encountered an issue.
    /// This can come in the form of network errors
    /// or the wrong has returning
    Failed,
    /// If we are rolling back the update failed to apply
    /// the new files to disk.
    RollingBack,
    /// The update package or installer is downloading.
    Downloading,
    /// The update is being validated by hashing the downloaded
    /// package or installer to check if it matches the manifest.
    Validating,
    /// The update is being written to disk.
    Applying,
    /// The update was applied successfully.
    Done,
}

#[derive(Default)]
pub struct ActiveUpdate {
    pub update_type: UpdateType,
    pub state: UpdateState,
    pub total_bytes: u64,
    pub downloaded_bytes: u64,
    pub branch: Branch,
}

#[derive(Deserialize, Default)]
pub struct Branch {
    /// The version of the active branch.
    pub version: String,
    /// The URL to the manifest file of the branches latest release.
    pub manifest_url: String,
    /// The full manifest for the branch.
    pub manifest: Option<Manifest>,
}

#[derive(Deserialize)]
pub struct Releases {
    /// The stable branch, used by default.
    pub stable: Branch,
    /// The beta branch, not used right now, but can be used for
    /// doing things such as 10% rollouts.
    pub beta: Branch,
    /// The nightly branch, not used now, but can be used for people
    /// who want bleeding edge changes.
    pub nightly: Branch,
}

#[derive(Deserialize)]
pub struct Manifest {
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
    pub url: String,
    /// A hash of the zip package, used to verify it downloaded properly.
    pub hash: String,
    /// A vector of all the files present inside the package.
    pub files: Vec<String>,
}

/// fetches all the available releases for each branch.
fn get_releases() -> Option<Releases> {
    match download_toml::<Releases>("http://local.vg:8080/Releases.toml") {
        Ok(r) => return Some(r),
        Err(e) => {
            // Send just in case its not a network error.
            sentry::capture_message(format!("{}", e).as_str(), sentry::Level::Error);
            // This is an unrecoverable issue.
            return None;
        }
    }
}

/// fetches the latest version of a branch and it's release manifest.
pub fn get_branch(branch: ReleaseBranch) -> Option<Branch> {
    let mut releases = match get_releases() {
        None => return None,
        Some(r) => r,
    };
    println!("pulling the latest release for the {:?} branch", branch);
    releases.stable.manifest = match download_toml::<Manifest>(&releases.stable.manifest_url) {
        Ok(m) => Some(m),
        Err(e) => {
            sentry::capture_message(format!("{}", e).as_str(), sentry::Level::Error);
            // This is an unrecoverable issue, so we return None
            // and present a generic error message.
            return None;
        }
    };
    Some(releases.stable)
}

pub fn download_release() {}

/// gets the latest Rainway 1.0 release. Used for installing Rainway.
pub fn get_latest_release_legacy() -> Option<ReleaseInfo> {
    if let Ok(info) = download_json::<ReleaseInfo>(env!("RAINWAY_RELEASE_URL")) {
        return Some(info);
    };
    None
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "PascalCase")]
/// DEPRECATED
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
