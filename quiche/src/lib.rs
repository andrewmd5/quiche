pub mod etc;
pub mod io;
pub mod net;
pub mod os;

pub mod bakery {

    use crate::etc::constants::BootstrapError;
    use crate::io::disk::{delete_dir_contents, get_dir_files, to_slash};
    use crate::io::hash::sha_256;
    use crate::io::zip::zip_with_progress;
    use crate::updater::{
        get_base_release_url, get_releases, Branch, Installer, Manifest, Package, ReleaseBranch,
        Releases,
    };
    use fs_extra::file::{copy, CopyOptions};
    use serde::Deserialize;
    use std::fs::read_dir;
    use std::io::{Error, ErrorKind};
    use std::{
        fs::{create_dir_all, read_to_string, write},
        path::{Path, PathBuf},
    };

    /// A recipe is used to craft a release from a given build.
    #[derive(Deserialize)]
    pub struct Recipe {
        /// the version of the release
        pub version: String,
        /// the full path to the MSI or EXE installer for the parent application.
        pub installer_path: PathBuf,
        /// the directory path to the files that makeup the release version.
        /// usually this will be digtally signed artifacts.
        pub package_source: PathBuf,
        /// the destination branch the release will be under
        pub branch: ReleaseBranch,
        /// the directory baked files will be written too. You should keep this the same between
        /// branches and versions. do not include the version number or branch name.
        pub output_dir: PathBuf,
    }

    pub struct Dinner {
        /// the baked Manifest of the target release which will serve as our
        /// manifest.toml in the final build
        pub manifest: Manifest,
        /// the baked Branch of the target release which will be blitted
        /// into Releases.toml
        pub branch: Branch,
    }

    impl Recipe {
        /// prepares a given recipe by checking that the provided flags are valid.
        /// it will also fix the slashes of paths, adding trailing slashes if needed.
        pub fn prepare(&mut self) -> Result<(), Error> {
            if self.version.is_empty() {
                return Err(Error::new(
                    ErrorKind::NotFound,
                    "The recipe version flag cannot be empty.",
                ));
            }
            self.installer_path = to_slash(&self.installer_path);
            if !self.installer_path.is_file() || !self.installer_path.exists() {
                return Err(Error::new(
                    ErrorKind::NotFound,
                    "The installer present in the recipe does not exist.",
                ));
            }
            self.package_source = to_slash(&self.package_source);
            if !self.package_source.is_dir() || !self.package_source.exists() {
                return Err(Error::new(
                    ErrorKind::NotFound,
                    "The recipe package source does not exist.",
                ));
            }
            self.output_dir.push(&self.branch.to_string());
            self.output_dir.push(&self.version);
            self.output_dir = to_slash(&self.output_dir);
            if !self.output_dir.exists() {
                create_dir_all(&self.output_dir)?;
                log::info!("created directory {}", &self.output_dir.display());
            } else {
                let output_dir = read_dir(&self.output_dir);
                delete_dir_contents(output_dir, &vec![])?;
                log::info!(
                    "deleted previous release found inside {}",
                    &self.output_dir.display()
                );
            }
            Ok(())
        }

        /// bakes a release by packaging and hashing all the relevant files
        /// once all the ingredients have been handled properly a Dinner struct
        /// is returned which contains the to-be released Manifest and Branch.
        pub fn bake(&self) -> Result<Dinner, BootstrapError> {
            log::info!(
                "building a release for {} on branch {}",
                self.version,
                self.branch
            );

            let branch_url = format!(
                "{}/{}/{}",
                get_base_release_url(),
                self.branch.to_string(),
                self.version
            );

            log::debug!("branch_url == {}", branch_url);

            // lets make the package
            log::info!("hashing the installer...");

            let installer_hash = sha_256(&self.installer_path).unwrap_or_default();
            if installer_hash.is_empty() {
                return Err(BootstrapError::RecipeBakeFailure(format!(
                    "Installer hash for {} is empty.",
                    self.installer_path.display()
                )));
            }

            let installer_url = format!("{}/installer.exe", branch_url);

            log::debug!(
                "installer_url == {}, installer_hash == {}",
                installer_url,
                installer_hash
            );

            log::info!("packaging the release files...");

            let func_test = |file: String| {
                log::info!("[DONE] {}", file);
            };

            let mut package_path = self.output_dir.clone();
            package_path.push("package.zip");

            if let Err(e) = zip_with_progress(&self.package_source, &package_path, func_test) {
                return Err(BootstrapError::RecipeBakeFailure(format!(
                    "Issue packaging {} due to unknown exception: {}",
                    &self.package_source.display(),
                    e
                )));
            }
            log::info!("hashing the release package...");

            let package_hash = sha_256(&package_path).unwrap_or_default();
            if package_hash.is_empty() {
                return Err(BootstrapError::RecipeBakeFailure(format!(
                    "package hash for {} is empty.",
                    package_path.display()
                )));
            }

            let package_url = format!("{}/package.zip", branch_url);

            log::debug!(
                "package_url == {}, package_hash == {}",
                package_url,
                package_hash
            );

            let files = get_dir_files(&self.package_source)?;

            if files.len() == 0 {
                return Err(BootstrapError::RecipeBakeFailure(format!(
                    "The package source {} contains zero files or we were unable to access the directory.",  
                    &self.package_source.display()
                )));
            }

            log::info!(
                "found {} files which will be included in this release.",
                files.len()
            );
            Ok(Dinner {
                branch: Branch {
                    manifest_url: format!("{}/manifest.toml", branch_url),
                    version: self.version.clone(),
                },
                manifest: Manifest {
                    version: self.version.clone(),
                    package: Package {
                        files: files,
                        hash: package_hash,
                        url: package_url,
                    },
                    installer: Installer {
                        url: installer_url,
                        hash: installer_hash,
                    },
                },
            })
        }

        /// using the baked Manifest and Branch structures we can prepare a release.
        /// an attempt is made to fetch the already published Releases.toml file
        /// so that it can be updated. If fetching it fails, then a brand new Releases file is made.
        /// all the relevant files will be written to their output once this function completes.
        pub fn stage(&self, dinner: Dinner) -> Result<(), BootstrapError> {
            log::info!(
                "attempting to stage the release for version {} on {}.",
                self.version,
                self.branch
            );
            let mut releases = match get_releases() {
                Ok(r) => {
                    log::info!("using default release host.");
                    r
                }
                Err(e) => {
                    log::warn!("Unable to fetch remote releases. {}", e);
                    log::warn!("Creating a Release file from scratch.");
                    Releases::default()
                }
            };
            log::info!("setting up the {} branch.", self.branch);
            match self.branch {
                ReleaseBranch::Stable => releases.stable = dinner.branch,
                ReleaseBranch::Beta => releases.beta = dinner.branch,
                ReleaseBranch::Nightly => releases.nightly = dinner.branch,
            }
            let releases_encoded = match toml::to_string(&releases) {
                Ok(c) => c,
                Err(e) => return Err(BootstrapError::RecipeStageFailure(e.to_string())),
            };
            log::debug!("encoded releases \n\n{}", &releases_encoded);
            let mut release_path = match self.output_dir.parent().unwrap().parent() {
                Some(p) => p.to_path_buf(),
                None => {
                    return Err(BootstrapError::RecipeStageFailure(
                        "Cannot locate parent for output directory.".to_string(),
                    ))
                }
            };
            release_path.push("Releases.toml");
            write(&release_path, &releases_encoded)?;
            log::info!("wrote Releases.toml to {}", &release_path.display());

            let manifest_encoded = match toml::to_string(&dinner.manifest) {
                Ok(c) => c,
                Err(e) => return Err(BootstrapError::RecipeStageFailure(e.to_string())),
            };
            log::debug!("encoded manifest \n\n{}", &manifest_encoded);

            let mut manifest_path = self.output_dir.clone();
            manifest_path.push("manifest.toml");
            write(&manifest_path, &manifest_encoded)?;
            log::info!("wrote release manifest to {}", &manifest_path.display());

            let mut options = CopyOptions::new();
            options.overwrite = true;

            let mut copied_installer_path = self.output_dir.clone();
            copied_installer_path.push("installer.exe");

            match copy(&self.installer_path, &copied_installer_path, &options) {
                Ok(_c) => _c,
                Err(e) => return Err(BootstrapError::RecipeStageFailure(e.to_string())),
            };
            log::info!(
                "copied the full installer to to {}",
                &copied_installer_path.display()
            );

            Ok(())
        }
    }

    impl From<&Path> for Recipe {
        /// turn a raw TOML string into a Recipe struct
        fn from(recipe_path: &Path) -> Self {
            if !recipe_path.exists() {
                panic!("The recipe at {} does not exist.", recipe_path.display());
            }
            if !recipe_path.is_file() {
                panic!("The provided recipe path is not a file.");
            }
            let contents =
                read_to_string(recipe_path).expect("Something went wrong reading the recipe file");
            if contents.is_empty() {
                panic!("The provided recipe at {} is empty.", recipe_path.display());
            }
            let recipe = toml::from_str::<Recipe>(&contents).expect("Unable to parse recipe");
            recipe
        }
    }
}

pub mod updater {

    use crate::etc::constants::BootstrapError;
    use crate::io::disk::{delete_dir_contents, dir_contains_all_files, get_filename};
    use crate::io::hash::sha_256;
    use crate::io::zip::unzip;
    use crate::net::http::{download_file, download_toml};
    use fs_extra::dir::{copy, move_dir, CopyOptions};
    use serde::{Deserialize, Serialize};

    use std::{
        env::{temp_dir, var},
        fs::{read_dir, remove_dir_all},
        path::Path,
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

    #[derive(Debug, Deserialize)]
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
        /// identifies if the current update is a full install or a patch.
        pub update_type: UpdateType,
        /// identifies the branch we are updating from.
        pub branch: ReleaseBranch,
        /// the manifest of the current update which contains information
        /// on the installer, files, and package hashes.
        pub manifest: Manifest,
        /// the temporary file path where downloaded packages will be written.
        pub temp_name: String,
        /// the currently installed version of the parent applicaton.
        pub current_version: String,
        /// the directory path where the parent application is installed
        /// and updates need to be written.
        pub install_path: String,
    }

    impl ActiveUpdate {
        /// fetches and sets the manifest for a given branch
        pub fn get_manifest(&mut self, branch: &ReleaseBranch) -> Result<(), BootstrapError> {
            let releases = get_releases()?;
            let manifest_url = match branch {
                ReleaseBranch::Stable => &releases.stable.manifest_url,
                ReleaseBranch::Beta => &releases.beta.manifest_url,
                ReleaseBranch::Nightly => &releases.nightly.manifest_url,
            };
            if manifest_url.is_empty() {
                return Err(BootstrapError::ReleaseLookupFailed(format!(
                    "Manifest URL missing the {} branch.",
                    branch
                )));
            }
            println!("pulling the latest release for the {:?} branch", branch);
            match download_toml::<Manifest>(&manifest_url) {
                Ok(m) => {
                    self.manifest = m;
                    return Ok(());
                }
                Err(e) => {
                    return Err(BootstrapError::ReleaseLookupFailed(format!(
                        "Failed to fetch branch {}. {}",
                        branch, e
                    )));
                }
            }
        }

        /// returns a list of all the files inside of a releases package.zip
        pub fn get_package_files(&self) -> Vec<String> {
            self.manifest.package.files.clone()
        }
        /// returns the temporary file path where downloaded packages will be written.
        pub fn get_temp_name(&self) -> String {
            self.temp_name.clone()
        }
        /// returns the remote version
        pub fn get_version(&self) -> String {
            self.manifest.version.clone()
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
                UpdateType::Install => self.manifest.installer.url.clone(),
                UpdateType::Patch => self.manifest.package.url.clone(),
            }
        }
        pub fn get_hash(&self) -> String {
            match self.update_type {
                UpdateType::Install => self.manifest.installer.hash.clone(),
                UpdateType::Patch => self.manifest.package.hash.clone(),
            }
        }
        /// creates a temporary file name for the update.
        pub fn set_temp_file(&mut self) {
            self.temp_name = format!("{}{}", self.get_hash(), self.get_ext())
        }

        /// Checks if the current installation is out of date.
        /// It does this by first checking if all the files listed in the manifest are present on disk.
        /// If all files are present, it then compares the remote and local version.
        /// Using this method bad installs/updates can be recovered.
        pub fn validate(&self) -> bool {
            if !validate_files(&self.install_path, &self.get_package_files()) {
                println!("We need to update because required files are missing.");
                return false;
            }
            return &self.current_version == &self.manifest.version;
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
    }

    #[derive(Serialize, Deserialize, Default)]
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

    #[derive(Serialize, Deserialize, Default)]
    pub struct Manifest {
        /// the version of the release
        pub version: String,
        /// The update package.
        pub package: Package,
        /// The full installer.
        pub installer: Installer,
    }

    #[derive(Serialize, Deserialize, Default)]
    pub struct Installer {
        /// The URL of the actual full installer.
        pub url: String,
        /// The hash of the installer to verify it downloaded properly.
        pub hash: String,
    }

    #[derive(Serialize, Deserialize, Default)]
    pub struct Package {
        /// The URL to the zip package containing all the new files.
        pub url: String,
        /// A hash of the zip package, used to verify it downloaded properly.
        pub hash: String,
        /// A vector of all the files present inside the package.
        pub files: Vec<String>,
    }

    pub fn get_base_release_url() -> String {
        env!("BASE_RELEASE_URL").to_string()
    }

    fn get_release_url() -> String {
        if let Ok(release_override) = var("RELEASE_OVERRIDE") {
            return release_override;
        }
        format!("{}{}", env!("BASE_RELEASE_URL"), env!("RELEASE_PATH"))
    }

    /// fetches all the available releases for each branch.
    pub fn get_releases() -> Result<Releases, BootstrapError> {
        download_toml::<Releases>(&get_release_url())
    }

    /// checks if all the files present in a vector exist in a given directory.
    fn validate_files(input: &String, target_files: &Vec<String>) -> bool {
        dir_contains_all_files(&Path::new(input), target_files)
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
        if let Err(e) = unzip(&download_path, &update_staging_path) {
            return Err(BootstrapError::InstallationFailed(format!(
                "Unable to extract update to {} due to issue: {}",
                update_staging_path.display(),
                e
            ))
            .to_string());
        }
        let current_exe = match std::env::current_exe() {
            Ok(exe) => get_filename(&exe),
            Err(e) => {
                return Err(BootstrapError::InstallationFailed(format!(
                    "Unable to locate current exe: {}",
                    e
                ))
                .to_string())
            }
        };
        
        //delete the install without deleting the root folder.
        let demo_dir = read_dir(&install_path);
        if let Err(e) = delete_dir_contents(demo_dir, &vec![current_exe]) {
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

        if let Err(e) = move_dir(&update_staging_path, &install_path, &options) {
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

        Ok("Rainway updated!".to_string())

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
