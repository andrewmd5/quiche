pub mod etc;
pub mod io;
pub mod net;
pub mod os;

pub mod updater {

    use crate::etc::constants::BootstrapError;
    use crate::io::disk::{delete_dir_contents, dir_contains_all_files, get_dir_files};
    use crate::io::hash::sha_256;
    use crate::io::zip::unzip;
    use crate::net::http::{download_file, download_toml};
    use fs_extra::dir::{copy, move_dir, CopyOptions};
    use serde::{Deserialize, Serialize};
    use version_compare::Version;

    use std::{
        env::temp_dir,
        fs::{read_dir, remove_dir_all},
        sync::{Arc, RwLock},
        thread,
        time::Duration,
    };

    #[derive(Debug)]
    pub enum UpdateType {
        /// An install requires us to run the full Rainway installer,
        /// becasuse Rainway itself is not installed.
        Install,
        /// A patch is applying new files to an existing installation.
        /// You have a hammer. After five months, you replace the head.
        /// After five more months, you replace the handle.
        /// Is it still the same hammer?
        Patch,
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
        pub branch: Branch,
        pub temp_name: String,
        pub current_version: String,
        pub install_path: String,
    }

    impl ActiveUpdate {
        pub fn get_package_files(&self) -> Vec<String> {
            self.branch.manifest.as_ref().unwrap().package.files.clone()
        }
        pub fn get_temp_name(&self) -> String {
            self.temp_name.clone()
        }
        pub fn get_version(&self) -> String {
            self.branch.version.clone()
        }
        pub fn get_ext(&self) -> String {
            match self.update_type {
                UpdateType::Install => ".exe",
                UpdateType::Patch => ".zip",
            }
            .to_string()
        }
        pub fn get_url(&self) -> String {
            match self.update_type {
                UpdateType::Install => self
                    .branch
                    .manifest
                    .as_ref()
                    .unwrap()
                    .installer
                    .url
                    .as_str(),
                UpdateType::Patch => self.branch.manifest.as_ref().unwrap().package.url.as_str(),
            }
            .to_string()
        }
        pub fn get_hash(&self) -> String {
            match self.update_type {
                UpdateType::Install => self
                    .branch
                    .manifest
                    .as_ref()
                    .unwrap()
                    .installer
                    .hash
                    .as_str(),
                UpdateType::Patch => self.branch.manifest.as_ref().unwrap().package.hash.as_str(),
            }
            .to_string()
        }
        /// Checks if the current installation is out of date.
        /// It does this by first checking if all the files listed in the manifest are present on disk.
        /// If all files are present, it then compares the remote and local version.
        /// Using this method bad installs/updates can be recovered.
        pub fn validate(&self) -> bool {
            if !validate_files(self.get_package_files(), self.install_path.clone()) {
                println!("We need to update because required files are missing.");
                return false;
            }

            if let Some(latest_ver) = Version::from(&self.branch.version) {
                if let Some(installed_ver) = Version::from(&self.current_version) {
                    if installed_ver < latest_ver {
                        return false;
                    } else {
                        return true;
                    }
                }
            }
            sentry::capture_message(
                format!(
                    "{}",
                    BootstrapError::VersionCheckFailed(
                        self.branch.version.to_string(),
                        self.current_version.clone()
                    )
                )
                .as_str(),
                sentry::Level::Error,
            );
            return false;
        }
    }

    #[derive(Default, Copy, Clone)]
    pub struct UpdateDownloadProgress {
        pub state: UpdateState,
        pub total_bytes: u64,
        pub downloaded_bytes: u64,
        pub faulted: bool,
    }

    #[derive(Serialize, Deserialize, Default)]
    pub struct Branch {
        /// The version of the active branch.
        pub version: String,
        /// The URL to the manifest file of the branches latest release.
        pub manifest_url: String,
        /// The full manifest for the branch.
        pub manifest: Option<Manifest>,
    }

    #[derive(Serialize, Deserialize)]
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

    impl Releases {
        pub fn to_string(&self) -> String {
            toml::to_string(&self).unwrap()
        }
    }

    #[derive(Serialize, Deserialize)]
    pub struct Manifest {
        /// The update package.
        pub package: Package,
        /// The full installer.
        pub installer: Installer,
    }

    #[derive(Serialize, Deserialize)]
    pub struct Installer {
        /// The URL of the actual full installer.
        pub url: String,
        /// The hash of the installer to verify it downloaded properly.
        pub hash: String,
    }

    #[derive(Serialize, Deserialize)]
    pub struct Package {
        /// The URL to the zip package containing all the new files.
        pub url: String,
        /// A hash of the zip package, used to verify it downloaded properly.
        pub hash: String,
        /// A vector of all the files present inside the package.
        pub files: Vec<String>,
    }

    /// fetches all the available releases for each branch.
    pub fn get_releases() -> Option<Releases> {
        match download_toml::<Releases>(env!("RAINWAY_RELEASE_URL")) {
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
                sentry::capture_message(
                    format!("Failed to fetch branch {}: {}", branch, e).as_str(),
                    sentry::Level::Error,
                );
                // This is an unrecoverable issue, so we return None
                // and present a generic error message.
                return None;
            }
        };
        Some(releases.stable)
    }

    /// checks if all the files present in a vector exist in a given directory.
    fn validate_files(target_files: Vec<String>, input: String) -> bool {
        dir_contains_all_files(target_files, &input)
    }

    /// checks if the downloaded file hash matches that of the one in the manifest.
    pub fn verify(remote_hash: String, input_file: String) -> Result<String, String> {
        let mut download_path = temp_dir();
        download_path.push(input_file);
        let result: Result<String, String> = Ok(String::default());
        let err: Result<String, String> = Err(BootstrapError::SignatureMismatch.to_string());
        if let Some(local_hash) = sha_256(&download_path) {
            match local_hash == remote_hash {
                true => return result,
                false => return err,
            }
        } else {
            return err;
        }
    }
    /// downloads a file from an HTTP server with a progress callback.
    pub fn download_with_callback<F>(
        url: String,
        output_file: String,
        callback: F,
    ) -> Result<String, String>
    where
        F: Fn(u64, u64) + Send + Sync + 'static,
    {
        let download_progress = UpdateDownloadProgress::default();
        let arc = Arc::new(RwLock::new(download_progress));
        let local_arc = arc.clone();
        let child = thread::spawn(move || loop {
            {
                let reader_lock = arc.clone();
                let reader = reader_lock.read().unwrap();
                if reader.faulted {
                    drop(reader);
                    break;
                }
                if reader.state == UpdateState::Downloading
                    && reader.total_bytes == reader.downloaded_bytes
                {
                    drop(reader);
                    break;
                }
                callback(reader.total_bytes, reader.downloaded_bytes);
                drop(reader);
            }
            thread::sleep(Duration::from_millis(16));
        });
        let mut download_path = temp_dir();
        download_path.push(output_file);

        let results = download_file(local_arc, &url, &download_path)
            .map_err(|err| format!("{}", err))
            .map(|output| format!("{}", output));
        let _res = child.join();
        results
    }

    /// applies an update package from a remote manifest.
    /// if any issues are encountered then the process will be rolled back.  
    pub fn apply(
        install_path: String,
        package_name: String,
        version: String,
    ) -> Result<String, String> {
        let mut download_path = temp_dir();
        download_path.push(package_name);
        let mut update_staging_path = temp_dir();
        update_staging_path.push(format!("Rainway_Stage_{}", &version));
        if update_staging_path.exists() {
            if let Err(e) = remove_dir_all(&update_staging_path) {
                let stage_clean_error = format!(
                    "Aborted update due to modification failure on stage {}: {}",
                    update_staging_path.display(),
                    e
                );
                return Err(BootstrapError::InstallationFailed(stage_clean_error).to_string());
            }
        }

        let mut backup_path = temp_dir();
        backup_path.push(format!("Rainway_Backup_{}", &version));
        if backup_path.exists() {
            if let Err(e) = remove_dir_all(&backup_path) {
                let backup_clean_error = format!(
                    "Aborted update due to modification failure on backup {}: {}",
                    backup_path.display(),
                    e
                );
                return Err(BootstrapError::InstallationFailed(backup_clean_error).to_string());
            }
        }
        let mut options = CopyOptions::new();
        options.copy_inside = true;
        options.content_only = true;
        options.overwrite = true;
        //make the backup
        if let Err(e) = copy(&install_path, &backup_path, &options) {
            let backup_error = format!(
                "Unable to backup installation to {}: {}",
                backup_path.display(),
                e
            );
            return Err(BootstrapError::InstallationFailed(backup_error).to_string());
        }
        //stage the update
        if !unzip(&download_path, &update_staging_path) {
            return Err(BootstrapError::InstallationFailed(format!(
                "Unable to extract update to {}",
                update_staging_path.display()
            ))
            .to_string());
        }

        //delete the install without deleting the root folder.
        let demo_dir = read_dir(&install_path);
        if let Err(e) = delete_dir_contents(demo_dir, vec!["pick.txt".to_string()]) {
            let delete_error = format!(
                "Unable to cleanup current installation located at {} due to: {}",
                &install_path, e
            );
            if let Ok(_e) = move_dir(&backup_path, &install_path, &options) {
                println!("rolled back update process.");
            } else {
                println!("failed to rollback update process.")
            }
            return Err(BootstrapError::InstallationFailed(delete_error).to_string());
        }

        //apply the update
        match move_dir(&update_staging_path, &install_path, &options) {
            Ok(_o) => {
                if let Some(fs) = get_dir_files(&install_path) {
                    for f in fs {
                        println!("{}", f);
                    }
                }
                return Ok("'Rainway Updated!'".to_string());
            }
            Err(e) => {
                let update_error_message = format!(
                    "Failed to apply update to {} from {}: {}",
                    &install_path,
                    &update_staging_path.display(),
                    e
                );
                if let Ok(_e) = move_dir(&backup_path, &install_path, &options) {
                    println!("rolled back update.");
                } else {
                    println!("failed to rollback update.")
                }
                return Err(BootstrapError::InstallationFailed(update_error_message).to_string());
            }
        }

        //dir_contains_all_files(package_files, &install_path);
    }

    /// Runs the full installer and waits for it to exit.
    /// The bootstrapper will not launch Rainway after this.
    /// The installer should be configured to launch post-install.
    pub fn install(installer_name: String) -> Result<String, String> {
        use std::os::windows::process::CommandExt;
        use std::process::Command;
        let mut download_path = temp_dir();
        download_path.push(installer_name);
        Command::new(download_path)
            .args(&[""])
            .creation_flags(0x08000000)
            .output()
            .map_err(|err| format!("{}", BootstrapError::InstallationFailed(err.to_string())))
            .map(|output| format!("'{}'", output.status.success()))
    }

}
