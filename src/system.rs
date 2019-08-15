use regex::Regex;
use std::path::Path;
use std::process::Command;
use std::str;
use winreg::enums::*;
use winreg::RegKey;

pub type UninstallersResult = Result<Vec<InstalledApp>, &'static str>;

/// dism.exe will return exit code 740 if it is launched
/// from a non-elevated process.
static ELEVATION_REQUIRED: i32 = 740;

#[derive(Debug, Clone, Default)]
/// Contains system information that was retreived from ```system::get_system_info()```
pub struct SystemInfo {
    /// Rainway is only supported on Windows 10, Server 2016, and Server 2019.
    pub is_supported: bool,
    /// Windows N and KN lack required codecs by default. If this is true ```system::needs_media_pack()```
    /// should be called.
    pub is_n_edition: bool,
    /// The product name of the OS, usually something like Windows 10 Pro.
    pub product_name: String,
    /// Rainway only works on x64 CPUs, so we check for this too.
    pub is_x64: bool,
}

#[derive(Debug, Clone, Default)]
//An installed app is represented here.
pub struct InstalledApp {
    pub uninstall_string: String,
    pub install_location: String,
    pub name: String,
}

#[derive(Clone, Default)]
/// Represents a DISM Operating System Package that is pulled from ```system::needs_media_pack()```
struct DismPackage {
    package_identity: String,
    state: String,
    release_type: String,
    install_time: String,
}

/// Returns a struct that contains basic info on the host system.
/// That includes whether Rainway supports its, the CPU architecture,
/// OS name, and if it is Windows N/KN.  
pub fn get_system_info() -> Result<SystemInfo, &'static str> {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let cur_ver = match hklm.open_subkey("SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion") {
        Err(_e) => return Err("Unable to open CurrentVersion under Windows NT."),
        Ok(o) => o,
    };
    let mut system_info = SystemInfo::default();
    let re = Regex::new("(N|KN)").unwrap();
    system_info.product_name = match cur_ver.get_value("ProductName") {
        Err(_error) => return Err("Unable to find ProductName value."),
        Ok(p) => p,
    };
    system_info.is_n_edition = re.is_match(&system_info.product_name);
    //this key will be missing on any non-Windows 10, Windows Server 2016 & 2019.
    let current_major_version_number: u32 = match cur_ver.get_value("CurrentMajorVersionNumber") {
        Err(_error) => 0,
        Ok(p) => p,
    };
    //We only support the above mentioned operating systems.
    system_info.is_supported = current_major_version_number == 10;
    system_info.is_x64 = is_x64()?;
    Ok(system_info)
}

/// Parses the registry to determine if the host OS is x32 or x64.
fn is_x64() -> Result<bool, &'static str> {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let environment = match hklm
        .open_subkey("SYSTEM\\CurrentControlSet\\Control\\Session Manager\\Environment")
    {
        Err(_e) => return Err("Problem opening registry the key for Environment"),
        Ok(o) => o,
    };
    let processor_architecture: String = match environment.get_value("PROCESSOR_ARCHITECTURE") {
        Ok(v) => v,
        Err(_error) => return Err("Problem opening PROCESSOR_ARCHITECTURE key"),
    };
    Ok(processor_architecture == "AMD64")
}

/// Determines if Windows N/KN users have the Media Feature Pack installed.
/// Windows N/KN do not have required codecs installed by default, so we need to prompt users.
/// This function requires the process to be elevated.
pub fn needs_media_pack() -> Result<bool, &'static str> {
    let tool = "dism";
    let args = ["/Online", "/Get-Packages"];

    let process = match Command::new(tool).args(&args).output() {
        Err(_e) => return Err("failed to execute dism"),
        Ok(o) => o,
    };
    let exit_code = process.status.code().expect("Could not unwrap error code.");
    if exit_code == ELEVATION_REQUIRED {
        return Err("Elevated permissions are required to run DISM.");
    }
    let mut packages: Vec<DismPackage> = Vec::new();
    let mut package = DismPackage::default();
    for line in String::from_utf8_lossy(&process.stdout).lines() {
        if line.contains("Package Identity") {
            package.package_identity = String::from(line.split(':').nth(1).unwrap().trim());
        } else if line.contains("State") {
            package.state = String::from(line.split(':').nth(1).unwrap().trim());
        } else if line.contains("Release Type") {
            package.release_type = String::from(line.split(':').nth(1).unwrap().trim());
        } else if line.contains("Install Time") {
            package.install_time = String::from(line.split(':').nth(1).unwrap().trim());
            packages.push(package.clone());
        }
    }
    for dism_package in packages {
        if dism_package.release_type == "Feature Pack" && dism_package.state == "Installed" {
            if dism_package
                .package_identity
                .contains("Microsoft-Windows-MediaFeaturePack")
            {
                return Ok(false);
            }
        }
    }
    Ok(true)
}
/// Derives if Rainway is currently installed based on
/// the list of installed applications for the current user.
pub fn is_rainway_installed() -> Result<bool, &'static str> {
    let uninstallers = match get_uninstallers() {
        Ok(u) => u,
        Err(error) => return Err(error),
    };
    //wow, I was wondering if there was an `any` trait like LINQ
    Ok(uninstallers.into_iter().any(|u| u.name == "Rainway"))
}

/// Runs the downloaded Rainway installer and waits for it to complete.
pub fn run_intaller(path: &Path) -> Result<bool, String> {
    let installer = match Command::new(path).args(&[""]).output() {
        Err(_e) => return Err(_e.to_string()),
        Ok(o) => o,
    };
    Ok(installer.status.success())
}

/// Returns a list of all the installed software for the current user.
fn get_uninstallers() -> UninstallersResult {
    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let uninstall_key =
        match hkcu.open_subkey("SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall") {
            Ok(u) => u,
            Err(_e) => return Err("Problem opening uninstall_key"),
        };
    let mut apps: Vec<InstalledApp> = Vec::new();
    for key in uninstall_key
        .enum_keys()
        .map(|x| x.unwrap())
        .filter(|x| !x.trim().is_empty())
    {
        if let Ok(install_key) = uninstall_key.open_subkey(key) {
            let mut app = InstalledApp::default();

            app.name = install_key
                .get_value("DisplayName")
                .unwrap_or(String::from(""));
            app.install_location = install_key
                .get_value("InstallLocation")
                .unwrap_or(String::from(""));
            app.uninstall_string = install_key
                .get_value("UninstallString")
                .unwrap_or(String::from(""));

            if !app.name.is_empty()
                && !app.install_location.is_empty()
                && !app.uninstall_string.is_empty()
            {
                apps.push(app);
            }
        }
    }
    Ok(apps)
}
