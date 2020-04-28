pub mod etc;
pub mod io;
pub mod net;
pub mod os;

pub mod bakery {

    use crate::etc::constants::BootstrapError;
    use crate::io::disk::{copy_file, delete_dir_contents, get_dir_files, to_slash};
    use crate::io::hash::sha_256;
    use crate::io::zip::zip_with_progress;
    use crate::updater::{
        get_base_release_url, get_releases, Branch, Installer, Manifest, Package, ReleaseBranch,
        Releases,
    };
    use serde::Deserialize;
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
                delete_dir_contents(&self.output_dir, &vec![])?;
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

            let mut copied_installer_path = self.output_dir.clone();
            copied_installer_path.push("installer.exe");

            if let Err(e) = copy_file(&self.installer_path, &copied_installer_path) {
                return Err(BootstrapError::RecipeStageFailure(e.to_string()));
            }
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
    use crate::io::disk::to_slash;
    use crate::io::disk::{
        copy_dir, delete_dir_contents, dir_contains_all_files, get_filename, move_dir, swap_files,
    };
    use crate::io::hash::sha_256;
    use crate::io::zip::unzip;
    use crate::net::http::{download_file, download_toml, post};
    use crate::os::files::{
        grant_full_permissions, take_ownership_of_dir, unblock_file, unblock_path,
    };
    use crate::os::windows::{
        create_reg_key, get_reg_key, get_uninstallers, set_uninstall_value, RegistryHandle,
    };
    use serde::{Deserialize, Serialize};
    use std::fs::{create_dir_all, remove_dir_all};

    use std::{
        env::{temp_dir, var},
        path::{Path, PathBuf},
    };

    /// a struct that represents information found in the uninstall key registry entry
    #[derive(Default, Clone)]
    pub struct InstallInfo {
        pub name: String,
        pub version: String,
        pub path: PathBuf,
        pub branch: ReleaseBranch,
        pub registry_key: String,
        pub registry_handle: RegistryHandle,
        pub id: String,
    }

    #[derive(Debug, PartialEq)]
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

    #[derive(Debug, Deserialize, Copy, Clone)]
    pub enum ReleaseBranch {
        Stable,
        Beta,
        Nightly,
    }
    #[derive(Default, Clone)]
    pub struct ActiveUpdate {
        /// identifies if the current update is a full install or a patch.
        pub update_type: UpdateType,
        /// the manifest of the current update which contains information
        /// on the installer, files, and package hashes.
        pub manifest: Manifest,
        /// the temporary file path where downloaded packages will be written.
        pub temp_name: String,
        /// the currently installed info of the parent applicaton.
        pub install_info: InstallInfo,
    }

    impl ActiveUpdate {
        /// fetches and sets the manifest for a given branch
        pub fn get_manifest(&mut self, branch: ReleaseBranch) -> Result<(), BootstrapError> {
            let releases = get_releases()?;
            let manifest_url = match branch {
                ReleaseBranch::Stable => &releases.stable.manifest_url,
                ReleaseBranch::Beta => &releases.beta.manifest_url,
                ReleaseBranch::Nightly => &releases.nightly.manifest_url,
            };
            if manifest_url.is_empty() {
                return Err(BootstrapError::ReleaseLookupFailed(format!(
                    "Manifest URL missing the {} branch.",
                    &branch
                )));
            }
            log::info!("pulling the latest release for the {:?} branch", branch);
            match download_toml::<Manifest>(&manifest_url) {
                Ok(m) => {
                    self.manifest = m;
                    return Ok(());
                }
                Err(e) => {
                    return Err(BootstrapError::ReleaseLookupFailed(format!(
                        "Failed to fetch branch {}. {}",
                        &branch, e
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
            if !validate_files(&self.install_info.path, &self.get_package_files()) {
                log::warn!("We need to update because required files are missing.");
                return false;
            }
            return &self.install_info.version == &self.manifest.version;
        }

        /// updates the version string used by the Add/Remove program menu on Windows
        /// we also use this to check if we need to update.
        pub fn update_display_version(&self) {
            if let Err(e) = set_uninstall_value(
                "DisplayVersion",
                &self.get_version(),
                &self.install_info.registry_key,
                self.install_info.registry_handle,
            ) {
                log::warn!("Unable to update display version: {}", e.to_string());
            }
        }

        /// allows the release branch to be changed to a new preferred.
        pub fn change_release_branch(&self, branch: ReleaseBranch) {
            if let Err(e) = set_uninstall_value(
                "QuicheBranch",
                &branch.to_string(),
                &self.install_info.registry_key,
                self.install_info.registry_handle,
            ) {
                log::warn!("Unable to update release branch: {}", e.to_string());
            }
        }

        fn store_installer_id(&mut self) {
            if let Some(id) = get_installer_id() {
                match set_rainway_key_value("SetupId", &id) {
                    Ok(_) => log::debug!("Set key successfully!"),
                    Err(e) => log::debug!("Unable to set key {}", e),
                };

                self.install_info.id = id;
            } else {
                log::debug!("Could not find chief tags for setupid id")
            }
        }

        pub fn store_event(new_state: RainwayAppState) {
            match set_rainway_key_value("SetupState", &(new_state as u32)) {
                Ok(_) => log::debug!("Set install state successfully!"),
                Err(e) => log::debug!("Unable to set install state {}", e),
            };
        }

        pub fn get_install_info() -> Result<InstallInfo, BootstrapError> {
            let uninstallers = get_uninstallers()?;
            let uninstaller = match uninstallers
                .into_iter()
                .find(|u| u.key == env!("UNINSTALL_KEY"))
            {
                Some(u) => u,
                None => return Err(BootstrapError::UninstallEntryMissing),
            };

            if uninstaller.version.is_empty() {
                return Err(BootstrapError::LocalVersionMissing);
            }
            if uninstaller.install_location.is_empty() {
                return Err(BootstrapError::InstallPathMissing);
            }

            let path = to_slash(&PathBuf::from(&uninstaller.install_location));
            // Knagie pointed out this probably should not be a fatal failure.
            // If the install path is known, but it was deleted, we can just recreate it.
            if !path.exists() {
                create_dir_all(&path)?;
            }

            // Check if we have an install key
            let setup_id = if let Ok(x) = get_rainway_key() {
                x.setup_id
            } else {
                log::debug!("First time setup");
                String::default()
            };

            let install_info = InstallInfo {
                version: uninstaller.version,
                branch: ReleaseBranch::from(uninstaller.branch),
                name: uninstaller.name,
                path,
                registry_key: uninstaller.key,
                registry_handle: uninstaller.handle,
                id: setup_id,
            };
            Ok(install_info)
        }

        /// retreives information on the current installed version of the parent software
        pub fn store_install_info(&mut self) -> Result<(), BootstrapError> {
            match ActiveUpdate::get_install_info() {
                Ok(install_info) => {
                    self.install_info = install_info;
                    Ok(())
                }
                Err(e) => Err(e),
            }
        }

        pub fn post_headers() -> std::collections::HashMap<&'static str, &'static str> {
            use std::collections::HashMap;

            // Headers that we want to send on post requests to the api

            [
                ("Origin", env!("API_ORIGIN")),
                ("Content-Type", "application/json"),
            ]
            .iter()
            .cloned()
            .collect()
        }

        // pub fn post_install_created(&self) {
        //     match post(
        //         env!("INSTALL_ENDPOINT"),
        //         format!(
        //             r#"{{"uuid":"{}", "version": "{}"}}"#,
        //             self.install_info.id, self.install_info.version,
        //         ),
        //         Some(ActiveUpdate::post_headers()),
        //     ) {
        //         Ok(s) => match s {
        //             hyper::StatusCode::OK => log::debug!("Posted install successfully"),
        //             x => log::debug!("Failed to post {:?}", x),
        //         },
        //         x => log::debug!("Failed to post {:?}", x),
        //     }
        // }

        pub fn post_update(&self, old_version: &str) {
            let new_version = &self.install_info.version;
            match post(
                env!("UPDATE_ENDPOINT"),
                format!(
                    r#"{{"uuid":"{}","version":"{}","lastVersion":"{}"}}"#,
                    self.install_info.id, self.install_info.version, old_version
                ),
                Some(ActiveUpdate::post_headers()),
            ) {
                Ok(s) => match s {
                    hyper::StatusCode::OK => log::debug!("Posted update successfully"),
                    x => log::debug!("Failed to post {:?}", x),
                },
                x => log::debug!("Failed to post {:?}", x),
            }
        }

        pub fn post_install(&self) {
            match post(
                env!("INSTALL_ENDPOINT"),
                format!(
                    r#"{{"uuid":"{}", "version": "{}"}}"#,
                    self.install_info.id, self.install_info.version,
                ),
                Some(ActiveUpdate::post_headers()),
            ) {
                Ok(s) => match s {
                    hyper::StatusCode::OK => log::debug!("Posted install successfully"),
                    x => log::debug!("Failed to post {:?}", x),
                },
                x => log::debug!("Failed to post {:?}", x),
            }
        }

        pub fn try_self_care(&mut self) -> Result<(), BootstrapError> {
            use std::fs::remove_file;
            let mut new_bootstrapper = self.install_info.path.clone();
            let mut old_bootstrapper = self.install_info.path.clone();
            let current_exe = get_filename(&std::env::current_exe()?);
            new_bootstrapper.push(format!("{}_new", current_exe));
            old_bootstrapper.push(format!("{}_old", current_exe));
            if old_bootstrapper.exists() {
                remove_file(&old_bootstrapper)?;
            }
            if !new_bootstrapper.exists() {
                return Ok(());
            }
            swap_files(&std::env::current_exe()?, &new_bootstrapper)?;
            Ok(())
        }
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

    #[derive(Serialize, Deserialize, Default, Clone)]
    pub struct Manifest {
        /// the version of the release
        pub version: String,
        /// The update package.
        pub package: Package,
        /// The full installer.
        pub installer: Installer,
    }

    #[derive(Serialize, Deserialize, Default, Clone)]
    pub struct Installer {
        /// The URL of the actual full installer.
        pub url: String,
        /// The hash of the installer to verify it downloaded properly.
        pub hash: String,
    }

    #[derive(Serialize, Deserialize, Default, Clone)]
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
    fn validate_files(input: &PathBuf, target_files: &Vec<String>) -> bool {
        dir_contains_all_files(&Path::new(input), target_files)
    }

    /// checks if the downloaded file hash matches that of the one in the manifest.
    pub fn verify(update: ActiveUpdate) -> Result<String, String> {
        let mut download_path = temp_dir();
        download_path.push(update.get_temp_name());
        log::info!("hashing {}", &download_path.display());
        let result: Result<String, String> = Ok(String::default());
        let err: Result<String, String> = Err(BootstrapError::SignatureMismatch.to_string());
        if let Some(local_hash) = sha_256(&download_path) {
            log::info!("finished hashing {}", &download_path.display());
            match local_hash == update.get_hash() {
                true => return result,
                false => return err,
            }
        } else {
            log::error!("failed to hash {}", &download_path.display());
            return err;
        }
    }
    /// downloads a file from an HTTP server with a progress callback.
    pub fn download_with_callback<F>(update: ActiveUpdate, callback: F) -> Result<String, String>
    where
        F: Fn(u64, u64) + Send + Sync + 'static,
    {
        use tokio::runtime::Runtime;
        let mut runtime = match Runtime::new() {
            Ok(rt) => rt,
            Err(e) => return Err(format!("{}", BootstrapError::from(e))),
        };
        let results = runtime.block_on(async {
            log::info!("download background thread started");
            let mut download_path = temp_dir();
            download_path.push(update.get_temp_name());
            let results = download_file(callback, &update.get_url(), &download_path)
                .await
                .map_err(|err| format!("{}", err))
                .map(|output| format!("{}", output));
            if results.is_ok() {
                log::info!("unblocking {}", &download_path.display());
                unblock_file(download_path);
            }
            results
        });
        drop(runtime);
        results
    }

    /// applies an update package from a remote manifest.
    /// if any issues are encountered then the process will be rolled back.  
    pub fn apply(update: ActiveUpdate) -> Result<String, String> {
        let mut download_path = temp_dir();
        download_path.push(update.get_temp_name());
        let mut update_staging_path = temp_dir();
        update_staging_path.push(format!("Rainway_Stage_{}", &update.get_version()));

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
        log::info!("current_exe == {}", &current_exe);
        let log_file = format!("{}", current_exe.replace(".exe", ".log"));
        let old_file = format!("{}_old", current_exe);
        let new_file = format!("{}_new", current_exe);
        let ignored_files = vec![current_exe, log_file, old_file, new_file];

        log::debug!("update_staging_path == {}", &update_staging_path.display());

        if update_staging_path.exists() {
            log::info!("update staging folder exist. attempting to clean.");
            if let Err(e) = remove_dir_all(&update_staging_path) {
                let stage_clean_error = format!(
                    "Aborted update due to modification failure on stage {}: {}",
                    update_staging_path.display(),
                    e
                );
                log::error!("{}", stage_clean_error);
                return Err(BootstrapError::InstallationFailed(stage_clean_error).to_string());
            }
        }

        let mut backup_path = temp_dir();
        backup_path.push(format!("Rainway_Backup_{}", &update.get_version()));

        log::debug!("backup_path == {}", &backup_path.display());
        if backup_path.exists() {
            log::info!("backup folder exist. attempting to clean.");
            if let Err(e) = remove_dir_all(&backup_path) {
                let backup_clean_error = format!(
                    "Aborted update due to modification failure on backup {}: {}",
                    backup_path.display(),
                    e
                );
                log::error!("{}", backup_clean_error);
                return Err(BootstrapError::InstallationFailed(backup_clean_error).to_string());
            }
        }
        //make the backup
        log::info!("attempting to create a backup of the current installation.");
        if let Err(e) = copy_dir(&update.install_info.path, &backup_path, &ignored_files) {
            let backup_error = format!(
                "Unable to backup installation to {}: {}",
                backup_path.display(),
                e
            );
            log::error!("{}", backup_error);
            return Err(BootstrapError::InstallationFailed(backup_error).to_string());
        }
        log::info!("backup completed.");
        log::info!("attempting to extract update package.");
        //stage the update
        if let Err(e) = unzip(&download_path, &update_staging_path) {
            let unzip_error = format!(
                "Unable to extract update to {} due to issue: {}",
                update_staging_path.display(),
                e
            );
            log::error!("{}", unzip_error);
            return Err(BootstrapError::InstallationFailed(unzip_error).to_string());
        }
        log::info!("update extracted to {}", &update_staging_path.display());

        //delete the install without deleting the root folder.
        log::info!(
            "attempting to delete all the contents of {}",
            &update.install_info.path.display()
        );
        // let demo_dir = read_dir(&update.install_info.path);

        if let Err(e) = delete_dir_contents(&update.install_info.path, &ignored_files) {
            let delete_error = format!(
                "Unable to cleanup current installation located at {} due to: {}",
                &update.install_info.path.display(),
                e
            );
            log::error!("{}", delete_error);
            log::warn!("attempting to roll back.");
            if let Err(e) = move_dir(&backup_path, &update.install_info.path, &ignored_files) {
                log::error!("failed to rollback update process. {}", e);
            }
            return Err(BootstrapError::InstallationFailed(delete_error).to_string());
        }
        log::info!("attempting to write updated files.");
        if let Err(e) = move_dir(
            &update_staging_path,
            &update.install_info.path,
            &ignored_files,
        ) {
            let update_error_message = format!(
                "Failed to apply update to {} from {}: {}",
                &update.install_info.path.display(),
                &update_staging_path.display(),
                e
            );
            log::error!("{}", update_error_message);
            if let Ok(_e) = move_dir(&backup_path, &update.install_info.path, &ignored_files) {
                log::warn!("rolled back update.");
            } else {
                log::error!("failed to rollback update.")
            }
            return Err(BootstrapError::InstallationFailed(update_error_message).to_string());
        }

        if let Ok(_o) = unblock_path(&update.install_info.path) {
            log::info!("unblocked the install path.");
        } else {
            log::info!("failed to unblock the install path");
        }

        if take_ownership_of_dir(&update.install_info.path) {
            log::info!("took ownership of the install path.");
        } else {
            log::info!("could take not ownership of the install path.");
        }

        if grant_full_permissions(&update.install_info.path) {
            log::info!("granted full permissions to the install path.");
        } else {
            log::info!("unable to grant full permissions to the install path.");
        }

        log::info!("update went off without a hitch.");

        let old_version = &update.install_info.version;

        update.update_display_version();

        update.post_update(old_version);

        Ok("Rainway updated!".to_string())
        //dir_contains_all_files(package_files, &install_path);
    }

    /// Runs the full installer and waits for it to exit.
    /// The bootstrapper will not launch Rainway after this.
    /// The installer should be configured to launch post-install.
    pub fn install(update: &mut ActiveUpdate) -> Result<String, String> {
        use std::os::windows::process::CommandExt;
        use std::process::Command;
        let mut download_path = temp_dir();
        download_path.push(update.get_temp_name());
        log::info!("running {}", &download_path.display());
        let results = Command::new(download_path)
            .args(&["/qn"])
            .creation_flags(0x08000000)
            .output()
            .map_err(|err| BootstrapError::InstallationFailed(err.to_string()).to_string())
            .map(|output| {
                String::from_utf8_lossy(&output.stdout)
                    .to_owned()
                    .to_string()
            });
        if let Ok(output) = &results {
            log::info!("{}", output);
        } else {
            log::warn!("No output");
        }

        // Write the install id to registry
        // along with what happened

        update.store_installer_id();
        update.post_install();
        ActiveUpdate::store_event(RainwayAppState::Activate);

        results
    }

    /// Derives if Rainway is currently installed based on
    /// the list of installed applications for the current user.
    pub fn is_installed() -> Result<bool, BootstrapError> {
        let uninstallers = get_uninstallers().unwrap_or(Vec::new());
        Ok(uninstallers
            .into_iter()
            .any(|u| u.key == env!("UNINSTALL_KEY")))
    }

    fn get_installer_id() -> Option<String> {
        use std::fs::File;
        use std::io::{prelude::*, BufReader};
        let exe = match std::env::current_exe() {
            Ok(e) => e,
            Err(_) => return None,
        };

        let setup = match File::open(exe) {
            Ok(f) => f,
            Err(_) => return None,
        };

        fn find2(start: usize, haystack: &[u8], needle: &[u8]) -> Option<usize> {
            (&haystack[start..])
                .windows(needle.len())
                .position(|window| window == needle)
        }

        let chief_bytes = &"<chief>".to_owned().into_bytes();
        let end_chief_bytes = &"</chief>".to_owned().into_bytes();

        let mut buffer = Vec::new();
        let mut reader = BufReader::new(setup);

        if let Ok(size) = reader.read_to_end(&mut buffer) {
            let mut cur = 0 as usize;
            while cur < buffer.len() {
                let start = find2(cur, &buffer, chief_bytes);
                let end = find2(cur, &buffer, end_chief_bytes);

                match (start, end) {
                    (Some(start), Some(end)) => {
                        let start = start + cur;
                        let end = end + cur;
                        if end > start && end - start > chief_bytes.len() {
                            let s = &buffer[start + chief_bytes.len()..end];
                            if let Ok(s) = String::from_utf8(s.to_vec()) {
                                return Some(s);
                            }

                            return None;
                        } else {
                            cur = end + 2;
                            continue;
                        }
                    }

                    _ => {
                        return None;
                    }
                }
            }
        }
        None
    }

    pub enum RainwayAppState {
        Nothing = 0,
        Activate = 1,
        Update = 2,
        Deactivate = 3,
    }

    impl From<u32> for RainwayAppState {
        fn from(x: u32) -> Self {
            match x {
                1 => RainwayAppState::Activate,
                2 => RainwayAppState::Update,
                3 => RainwayAppState::Deactivate,
                _ => RainwayAppState::Nothing,
            }
        }
    }

    pub struct RainwayApp {
        pub setup_id: String,
        pub install_state: RainwayAppState,
    }

    pub fn get_rainway_key() -> Result<RainwayApp, BootstrapError> {
        let u_key = env!("RAINWAY_KEY");

        let key = match create_reg_key(RegistryHandle::CurrentUser, u_key) {
            Err(_e) => return Err(BootstrapError::RegistryKeyNotFound(u_key.to_string())),
            Ok(x) => x,
        };

        let app = RainwayApp {
            setup_id: key.get_value("SetupId").unwrap_or_default(),
            install_state: RainwayAppState::from(
                key.get_value::<u32, &str>("SetupState")
                    .unwrap_or(RainwayAppState::Nothing as u32),
            ),
        };

        Ok(app)
    }

    fn set_rainway_key_value<N: AsRef<std::ffi::OsStr>, T: winreg::types::ToRegValue>(
        subkey: N,
        value: &T,
    ) -> Result<(), BootstrapError> {
        let u_key = env!("RAINWAY_KEY");

        let key = match get_reg_key(RegistryHandle::CurrentUser, u_key) {
            Err(_e) => return Err(BootstrapError::RegistryKeyNotFound(u_key.to_string())),
            Ok(x) => x,
        };

        match key.set_value(subkey, value) {
            Ok(_) => Ok(()),
            Err(e) => Err(BootstrapError::UnableToSetRegKey(u_key.to_string())),
        }
    }

    pub fn post_deactivate(setup_id: String, version: String) {
        let headers = ActiveUpdate::post_headers();

        match post(
            env!("DEACTIVATE_ENDPOINT"),
            format!(r#"{{"uuid":"{}", "version": "{}"}}"#, setup_id, version,),
            Some(ActiveUpdate::post_headers()),
        ) {
            Ok(s) => match s {
                hyper::StatusCode::OK => log::debug!("Posted deactivate successfully"),
                x => log::debug!("Failed to post {:?}", x),
            },
            x => log::debug!("Failed to post {:?}", x),
        }
    }
}
