use crate::ui::messagebox::show_error;
use quiche::etc::constants::BootstrapError;
use quiche::os::process::get_processes;
use quiche::os::service::{install_service, service_exist, start_service, WindowsService};
use quiche::os::windows::{
    detach_rdp_session, get_dotnet_framework_version, get_system_info, needs_media_pack,
};
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

pub fn get_installer_id() -> Option<String> {
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

    fn find2(start: usize, haystack: &Vec<u8>, needle: &Vec<u8>) -> Option<usize> {
        let n = needle.as_slice();
        (&haystack[start..])
            .windows(needle.len())
            .position(|window| window == n)
    }

    let chief_bytes = &"<chief>".to_owned().into_bytes();
    let end_chief_bytes = &"</chief>".to_owned().into_bytes();

    let mut buffer = Vec::new();
    let mut reader = BufReader::new(setup);
    if let size = reader.read_to_end(&mut buffer) {
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

pub fn store_installer_id(info: &quiche::updater::InstallInfo) {
    if let Some(id) = get_installer_id() {
        use quiche::os::windows;
        match windows::set_uninstall_value("SetupId", &id, &info.registry_key, info.registry_handle)
        {
            Err(e) => log::debug!("Unable to set setupid value in registry {}", e),
            _ => log::debug!("Set setupid sucessfully"),
        }
    } else {
        log::debug!("Could not find chief tags for setupid id")
    }
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

    if !system_info.is_supported {
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
