use crate::etc::constants::BootstrapError;
use crate::os::windows::get_uninstallers;
use std::process;
use sysinfo::{ProcessExt, Signal, SystemExt};
use version_compare::{CompOp, Version, VersionCompare};

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

pub fn is_outdated(remote_version: &String) -> Option<bool> {
    let installed_version = get_installed_version().unwrap_or_default();
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
    use sysinfo::{ProcessExt, SystemExt};
    let sys = sysinfo::System::new();
    let current_pid = process::id();
    for (pid, proc_) in sys.get_process_list() {
        if proc_.name() == "rainway_bootstrapper.exe" && (*pid as u32) != current_pid {
            return Err(BootstrapError::BootstrapperExist);
        }
    }
    Ok(())
}

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