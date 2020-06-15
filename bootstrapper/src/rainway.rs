use crate::ui::messagebox::show_error;
use quiche::etc::constants::BootstrapError;
use quiche::os::process::get_processes;
use quiche::os::service::{install_service, service_exist, start_service, WindowsService};
use quiche::os::windows::{detach_rdp_session, get_dotnet_framework_version, get_system_info, WindowsVersion};
use std::path::PathBuf;
use std::process;
/// returns an error if the bootstrapper is already open
pub fn error_on_duplicate_session() -> Result<(), BootstrapError> {
    if let Some(process_list) = get_processes() {
        let current_pid = process::id();
        for process in process_list {
            if process.name() == format!("{}.exe", env!("CARGO_PKG_VERSION"))
                && process.id() != current_pid
            {
                return Err(BootstrapError::BootstrapperExist);
            }
        }
    }
    Ok(())
}

/// launches the Rainway service (Radar)
pub fn launch_rainway(install_path: &PathBuf) {
    if detach_rdp_session() {
        log::info!("Session was detached");
    } else {
        log::info!("Failed or not need to detach session.");
    }
    if !service_exist(env!("RAINWAY_SERVICE")) {
        let mut exe = install_path.to_path_buf();
        exe.push("radar\\Radar.exe");
        if exe.exists() && exe.is_file() {
            let service = WindowsService {
                name: env!("RAINWAY_SERVICE").to_string(),
                display_name: "Rainway Radar".to_string(),
                arguments: vec![],
                executable_path: exe,
            };
            match install_service(service) {
                Ok(s) => log::info!("Rainway service installed: {}", s),
                Err(e) => {
                    show_error("Rainway Startup Failure", format!("{}", e));
                    sentry::capture_message(format!("{}", e).as_str(), sentry::Level::Error);
                }
            }
        }
    }
    if service_exist(env!("RAINWAY_SERVICE")) {
        match start_service(env!("RAINWAY_SERVICE")) {
            Ok(s) => log::info!("Rainway service started: {}", s),
            Err(e) => {
                show_error("Rainway Startup Failure", format!("{}", e));
                sentry::capture_message(format!("{}", e).as_str(), sentry::Level::Error);
            }
        }
    }
}

/// kills all associated Rainway processes
pub fn kill_rainway() {
    if let Some(process_list) = get_processes() {
        for process in process_list {
            if process.name() == "Rainway.exe"
                || process.name() == "CefSharp.BrowserSubprocess.exe"
                || process.name() == "Radar.exe"
                || process.name() == "LaunchRainway.exe"
                || process.name() == "RainwayInstaller.exe"
            {
                if process.kill() {
                    log::info!("Rainway process {} terminated", process.name());
                }
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

    if system_info.windows_version != WindowsVersion::Windows10 {
        log::error!("{} is unsupported", system_info.product_name);
        return Err(BootstrapError::WindowsVersionUnsupported);
    }

    if system_info.is_n_edition {
        log::warn!("Windows N detected.");
        /*if needs_media_pack()? {
            log::error!("Windows Media Pack is not installed.");
            return Err(BootstrapError::NeedWindowsMediaPack(
                system_info.product_name,
            ));
        }*/
    }

    let dotnet = get_dotnet_framework_version();
    if dotnet.is_none() {
        return Err(BootstrapError::NeedDotNetFramework);
    }
    let dotnet_version = dotnet.unwrap_or_default();
    if dotnet_version < 461808 {
        return Err(BootstrapError::NeedDotNetFramework);
    }
    log::info!("The current .NET Framework version is: {}", dotnet_version);
    log::info!("current system is compatible.");
    Ok(())
}
