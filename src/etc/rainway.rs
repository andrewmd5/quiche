use crate::etc::constants::BootstrapError;
use crate::os::service::start_service;
use crate::os::windows::get_uninstallers;
use crate::ui::messagebox::show_error;
use crate::updater::validate_files;
use std::process;
use sysinfo::{ProcessExt, Signal, SystemExt};
use version_compare::Version;

/// Derives if Rainway is currently installed based on
/// the list of installed applications for the current user.
pub fn is_installed() -> Result<bool, BootstrapError> {
    let uninstallers = get_uninstallers()?;
    //wow, I was wondering if there was an `any` trait like LINQ
    Ok(uninstallers.into_iter().any(|u| u.name == "Rainway"))
}

/// TODO pull this from the registry
fn get_installed_version() -> Option<String> {
    Some(String::from("1.0.0"))
}

/// TODO pull this from the registry
/// THIS NEEDS A TRAILING SLASH
pub fn get_install_path() -> Option<String> {
    let registry_path = String::from("E:\\UpdateTest\\InstalledFolder\\");
    Some(registry_path)
}

/// Checks if the current Rainway installation is out of date.
/// It does this by first checking if all the files listed in the manifest are present on disk.
/// If all files are present, it then compares the remote and local version.
/// Using this method bad installs/updates can be recovered.
pub fn is_outdated(remote_version: &String, files: Vec<String>) -> Option<bool> {
    let install_path = match get_install_path() {
        Some(v) => v,
        None => return None,
    };
    if !validate_files(files, install_path) {
        println!("We need to update because required files are missing.");
        return Some(true);
    }
    let installed_version = match get_installed_version() {
        Some(v) => v,
        None => return None,
    };
    if let Some(latest_ver) = Version::from(&remote_version) {
        if let Some(installed_ver) = Version::from(&installed_version) {
            if installed_ver < latest_ver {
                return Some(true);
            } else {
                return Some(false);
            }
        }
    }
    sentry::capture_message(
        format!(
            "{}",
            BootstrapError::VersionCheckFailed(remote_version.to_string(), installed_version)
        )
        .as_str(),
        sentry::Level::Error,
    );
    None
}

pub fn error_on_duplicate_session() -> Result<(), BootstrapError> {
    let sys = sysinfo::System::new();
    let current_pid = process::id();
    for (pid, proc_) in sys.get_process_list() {
        if proc_.name() == "rainway_bootstrapper.exe" && (*pid as u32) != current_pid {
            return Err(BootstrapError::BootstrapperExist);
        }
    }
    Ok(())
}

/// launches the Rainway service (Radar)
pub fn launch_rainway() {
    match start_service(env!("RAINWAY_SERVICE")) {
        Ok(s) => println!("Rainway started: {}", s),
        Err(e) => {
            show_error("Rainway Startup Failure", format!("{}", e));
            sentry::capture_message(format!("{}", e).as_str(), sentry::Level::Error);
        }
    }
}

/// kills any process that is related to Rainway
pub fn kill_rainway_processes() {
    let sys = sysinfo::System::new();
    for (_pid, proc_) in sys.get_process_list() {
        if proc_.name() == "Rainway.exe" || proc_.name() == "CefSharp.BrowserSubprocess.exe" {
            if proc_.kill(Signal::Kill) {
                println!("Killed {}", proc_.name());
            }
        }
    }
}
