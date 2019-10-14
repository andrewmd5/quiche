use crate::etc::constants::BootstrapError;
use regex::Regex;
use winreg::enums::KEY_ALL_ACCESS;
use winreg::types::ToRegValue;
use winreg::HKEY;

use winreg::enums::HKEY_LOCAL_MACHINE;
use winreg::RegKey;

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

/// Windows use the opaque handle scheme that most operating systems use.
/// When requesting resources from the operating system, you are given a "handle" or cookie that represents the real object.
/// By supplying one of these handles to the registry we can scoped data.
#[derive(Debug, Clone, Copy)]
#[allow(overflowing_literals)]
pub enum RegistryHandle {
    CurrentUser = 0x80000001i32 as isize,
    LocalMachine = 0x80000002i32 as isize,
}

impl Default for RegistryHandle {
    fn default() -> Self {
        RegistryHandle::CurrentUser
    }
}

#[derive(Debug, Clone, Default)]
//An installed app is represented here.
pub struct InstalledApp {
    pub uninstall_string: String,
    pub install_location: String,
    pub name: String,
    pub version: String,
    pub branch: String,
    pub handle: RegistryHandle,
    pub key: String,
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
pub fn get_system_info() -> Result<SystemInfo, BootstrapError> {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let version_path = "SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion";
    let cur_ver = match hklm.open_subkey(version_path) {
        Err(_e) => {
            return Err(BootstrapError::RegistryKeyNotFound(
                version_path.to_string(),
            ))
        }
        Ok(o) => o,
    };
    let mut system_info = SystemInfo::default();
    let re = Regex::new("(N|KN)").unwrap();
    let pn = "ProductName";
    system_info.product_name = match cur_ver.get_value(pn) {
        Err(_error) => return Err(BootstrapError::RegistryValueNotFound(pn.to_string())),
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
fn is_x64() -> Result<bool, BootstrapError> {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let env = "SYSTEM\\CurrentControlSet\\Control\\Session Manager\\Environment";
    let environment = match hklm.open_subkey(env) {
        Err(_e) => return Err(BootstrapError::RegistryKeyNotFound(env.to_string())),
        Ok(o) => o,
    };
    let arch_key = "PROCESSOR_ARCHITECTURE";
    let processor_architecture: String = match environment.get_value(arch_key) {
        Ok(v) => v,
        Err(_error) => return Err(BootstrapError::RegistryValueNotFound(arch_key.to_string())),
    };
    Ok(processor_architecture == "AMD64")
}

/// Determines if Windows N/KN users have the Media Feature Pack installed.
/// Windows N/KN do not have required codecs installed by default, so we need to prompt users.
/// This function requires the process to be elevated.
pub fn needs_media_pack() -> Result<bool, BootstrapError> {
    use std::os::windows::process::CommandExt;
    use std::process::Command;
    let tool = "dism";
    let args = ["/Online", "/Get-Packages"];

    let process = match Command::new(tool)
        .args(&args)
        .creation_flags(0x08000000)
        .output()
    {
        Err(e) => return Err(BootstrapError::DismFailed(e.to_string())),
        Ok(o) => o,
    };
    let exit_code = process.status.code().expect("Could not unwrap error code.");
    if exit_code == ELEVATION_REQUIRED {
        return Err(BootstrapError::ElevationRequired);
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

pub fn set_uninstall_value<T: ToRegValue>(
    name: &str,
    value: &T,
    sub_key: &String,
    handle: RegistryHandle,
) -> Result<(), BootstrapError> {
    let u_key = "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall";
    let hkey = RegKey::predef(handle as isize as HKEY);
    let uninstall_key = match hkey.open_subkey(u_key) {
        Ok(u) => u,
        Err(_e) => return Err(BootstrapError::RegistryKeyNotFound(u_key.to_string())),
    };

    let install_key = match uninstall_key.open_subkey_with_flags(&sub_key, KEY_ALL_ACCESS) {
        Ok(k) => k,
        Err(_e) => return Err(BootstrapError::RegistryKeyNotFound(sub_key.to_string())),
    };
    if let Err(e) = install_key.set_value(name, value) {
        return Err(BootstrapError::RegistryValueNotFound(e.to_string()));
    }
    Ok(())
}

/// Returns a list of all the installed software for the current user and local machine
pub fn get_uninstallers() -> Result<Vec<InstalledApp>, BootstrapError> {
    let mut uninstallers = get_uninstallers_from_key(RegistryHandle::CurrentUser)?;
    uninstallers.extend(get_uninstallers_from_key(RegistryHandle::LocalMachine)?);
    Ok(uninstallers)
}

/// Returns a list of uninstallers for a given registry key
fn get_uninstallers_from_key(handle: RegistryHandle) -> Result<Vec<InstalledApp>, BootstrapError> {
    let u_key = "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall";
    let hkey = RegKey::predef(handle as isize as HKEY);
    let uninstall_key = match hkey.open_subkey(u_key) {
        Ok(u) => u,
        Err(_e) => return Err(BootstrapError::RegistryKeyNotFound(u_key.to_string())),
    };
    let mut apps: Vec<InstalledApp> = Vec::new();
    for key in uninstall_key
        .enum_keys()
        .map(|x| x.unwrap())
        .filter(|x| !x.trim().is_empty())
    {
        if let Ok(install_key) = uninstall_key.open_subkey(&key) {
            let mut app = InstalledApp::default();

            app.name = install_key.get_value("DisplayName").unwrap_or_default();
            app.install_location = install_key.get_value("InstallLocation").unwrap_or_default();
            app.uninstall_string = install_key.get_value("UninstallString").unwrap_or_default();
            app.version = install_key.get_value("DisplayVersion").unwrap_or_default();
            app.branch = install_key.get_value("QuicheBranch").unwrap_or_default();
            app.handle = handle.clone();
            app.key = key;

            if !app.name.is_empty()
                && !app.install_location.is_empty()
                && !app.uninstall_string.is_empty()
                && !app.version.is_empty()
            {
                apps.push(app);
            }
        }
    }
    Ok(apps)
}
