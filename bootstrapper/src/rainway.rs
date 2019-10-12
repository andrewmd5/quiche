use crate::ui::messagebox::show_error;
use quiche::etc::constants::BootstrapError;
use quiche::os::service::start_service;
use quiche::os::windows::{get_system_info, needs_media_pack};

use std::process;
use sysinfo::{ProcessExt, Signal, SystemExt};

/// returns an error if the bootstrapper is already open
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
        Ok(s) => log::info!("Rainway started: {}", s),
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
                log::info!("Killed {}", proc_.name());
            }
        }
    }
}

/// checks if the current system is compatible with Rainway
/// returns an error for the condition that is not met.
pub fn check_system_compatibility() -> Result<(), BootstrapError> {
    let system_info = get_system_info()?;

    if !system_info.is_x64 {
        log::error!("architecture unsupported");
        return Err(BootstrapError::ArchitectureUnsupported);
    }

    if !system_info.is_supported {
        log::error!("{} is unsupported", system_info.product_name);
        return Err(BootstrapError::WindowsVersionUnsupported);
    }

    if system_info.is_n_edition {
        log::warn!("Windows N detected.");
        if needs_media_pack()? {
            log::error!("Windows Media Pack is not installed.");
            return Err(BootstrapError::NeedWindowsMediaPack(
                system_info.product_name,
            ));
        }
    }
    log::info!("current system is compatible.");
    Ok(())
}
