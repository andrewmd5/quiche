use crate::etc::constants::BootstrapError;
use crate::os::windows::get_uninstallers;
use std::path::Path;
use std::process;
use std::process::Command;

/// Derives if Rainway is currently installed based on
/// the list of installed applications for the current user.
pub fn is_installed() -> Result<bool, BootstrapError> {
    let uninstallers = get_uninstallers()?;
    //wow, I was wondering if there was an `any` trait like LINQ
    Ok(uninstallers.into_iter().any(|u| u.name == "Rainway"))
}

/// TODO pull this from the registry
pub fn get_version() -> String {
    String::from("1.0.0")
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

/// Runs the downloaded Rainway installer and waits for it to complete.
pub fn run_intaller(path: &Path) -> Result<bool, BootstrapError> {
    let installer = match Command::new(path).args(&[""]).output() {
        Err(e) => return Err(BootstrapError::InstallationFailed(e.to_string())),
        Ok(o) => o,
    };
    Ok(installer.status.success())
}