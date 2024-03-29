use crate::etc::constants::BootstrapError;
use crate::os::process::get_current_process;
use crate::os::winver::{
    is_windows10_or_greater, is_windows7_or_greater, is_windows8_or_greater,
    is_windows8_point1_or_greater, is_windows_server, is_windows_vista_or_greater,
    is_windows_xpor_greater,
};
use regex::Regex;
use std::env::var_os;
use std::path::PathBuf;
use winapi::um::winnt::KEY_READ;
use winapi::um::winnt::KEY_WOW64_64KEY;
use winapi::um::winuser::{GetSystemMetrics, SM_REMOTESESSION};
use winreg::enums::HKEY_LOCAL_MACHINE;
use winreg::enums::KEY_ALL_ACCESS;
use winreg::types::ToRegValue;
use winreg::RegKey;
use winreg::HKEY;
/// dism.exe will return exit code 740 if it is launched
/// from a non-elevated process.
static ELEVATION_REQUIRED: i32 = 740;

#[derive(Debug, Clone, Default)]
/// Contains system information that was retreived from ```system::get_system_info()```
pub struct SystemInfo {
    /// Rainway is only supported on Windows 10, Server 2016, and Server 2019.
    pub windows_version: WindowsVersion,
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
#[derive(Debug, Clone, Copy, PartialEq)]
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
    //We only support the above mentioned operating systems.
    system_info.windows_version = match get_windows_version() {
        Ok(v) => v,
        Err(e) => return Err(e),
    };
    // if current_major_version_number == 10;
    system_info.is_x64 = is_x64();
    Ok(system_info)
}

/// Parses the registry to determine if the host OS is x32 or x64.
fn is_x64() -> bool {
    use winapi::um::sysinfoapi;
    let mut out: sysinfoapi::SYSTEM_INFO = unsafe { std::mem::MaybeUninit::zeroed().assume_init() };
    unsafe { sysinfoapi::GetNativeSystemInfo(&mut out as sysinfoapi::LPSYSTEM_INFO) };
    match unsafe { out.u.s() }.wProcessorArchitecture {
        //x64 (AMD or Intel)
        winapi::um::winnt::PROCESSOR_ARCHITECTURE_AMD64 => true,
        _ => false,
    }
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
/// determines if your application is running in a remote session.
/// if the ID of the current session in which the application is running is the same as in the registry key,
/// the application is running in a local session. Sessions identified as remote session in this way include
/// remote sessions that use RemoteFX vGPU.
pub fn is_current_session_remoteable() -> bool {
    unsafe {
        if GetSystemMetrics(SM_REMOTESESSION) != 0 {
            return true;
        }
        // RemoteFX vGPU now we need to check for RemoteFX vGPU
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        let terminal_server_key = "SYSTEM\\CurrentControlSet\\Control\\Terminal Server";
        let terminal_server = match hklm.open_subkey(terminal_server_key) {
            Err(_e) => {
                return false;
            }
            Ok(o) => o,
        };
        let glass_session_id: u32 = match terminal_server.get_value("GlassSessionId") {
            Err(_error) => return false,
            Ok(p) => p,
        };
        if let Some(p) = get_current_process() {
            return p.session_id() != glass_session_id;
        }
        false
    }
}
/// if we are running in a remote sesssion, detach the RDP session so applications can
/// be launched into a non-system context from a service.
pub fn detach_rdp_session() -> bool {
    use std::os::windows::process::CommandExt;
    use std::process::Command;
    if is_current_session_remoteable() {
        let key = "WINDIR";
        let mut tscon_path = match var_os(key) {
            Some(val) => PathBuf::from(val), //add the Windows path
            None => return false,
        };
        tscon_path.push("System32");
        tscon_path.push("tscon.exe");
        if tscon_path.exists() && tscon_path.is_file() {
            log::info!("Found tscon at {}", &tscon_path.display());
            if let Some(p) = get_current_process() {
                let session_id = format!("{}", p.session_id());
                log::info!("Current session ID is: {}", session_id);
                match Command::new(&tscon_path)
                    .args(&[session_id, "/dest:console".to_string()])
                    .creation_flags(0x08000000)
                    .output()
                {
                    Ok(_o) => return true,
                    Err(_e) => return false,
                };
            }
        }
    }
    false
}

pub fn set_uninstall_value<T: ToRegValue>(
    name: &str,
    value: &T,
    sub_key: &String,
    handle: RegistryHandle,
) -> Result<(), BootstrapError> {
    let u_key = "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall";
    let hkey = RegKey::predef(handle as isize as HKEY);
    let uninstall_key = match hkey.open_subkey_with_flags(u_key, KEY_READ | KEY_WOW64_64KEY) {
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

pub fn get_dotnet_framework_version() -> Option<u32> {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    let u_key = "SOFTWARE\\Microsoft\\NET Framework Setup\\NDP\\v4\\Full";
    let full = match hklm.open_subkey(u_key) {
        Err(_e) => {
            return None; //.NET Framework 4.5 or later is not installed.
        }
        Ok(o) => o,
    };
    let version: u32 = match full.get_value("Release") {
        Err(_error) => return None,
        Ok(p) => p,
    };
    Some(version)
}

/// Returns a list of uninstallers for a given registry key
fn get_uninstallers_from_key(handle: RegistryHandle) -> Result<Vec<InstalledApp>, BootstrapError> {
    let u_key = "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall";
    let hkey = RegKey::predef(handle as isize as HKEY);
    let uninstall_key = match hkey.open_subkey_with_flags(u_key, KEY_READ | KEY_WOW64_64KEY) {
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

pub fn get_reg_key(handle: RegistryHandle, key: &str) -> Result<RegKey, BootstrapError> {
    let hkey = RegKey::predef(handle as isize as HKEY);

    match hkey.open_subkey_with_flags(key, KEY_ALL_ACCESS | KEY_WOW64_64KEY) {
        Ok(u) => Ok(u),
        Err(_e) => Err(BootstrapError::RegistryKeyNotFound(key.to_string())),
    }
}

pub fn delete_reg_key(handle: RegistryHandle, key: &str) {
    let hkey = RegKey::predef(handle as isize as HKEY);
    if let Ok(_) = hkey.delete_subkey_all(&key) {
        log::info!("Deleted key");
    }
}

pub fn create_reg_key(handle: RegistryHandle, key: &str) -> Result<RegKey, BootstrapError> {
    let hkey = RegKey::predef(handle as isize as HKEY);

    match hkey.open_subkey_with_flags(key, KEY_ALL_ACCESS | KEY_WOW64_64KEY) {
        Ok(u) => Ok(u),
        Err(_e) => match hkey.create_subkey_with_flags(key, KEY_ALL_ACCESS | KEY_WOW64_64KEY) {
            Ok((u, _)) => Ok(u),
            Err(_e) => Err(BootstrapError::RegistryKeyNotFound(key.to_string())),
        },
    }
}

pub fn get_windows_version() -> Result<WindowsVersion, BootstrapError> {
    if is_windows10_or_greater() {
        Ok(WindowsVersion::Windows10)
    } else if is_windows8_point1_or_greater() {
        Ok(WindowsVersion::Windows81)
    } else if is_windows8_or_greater() {
        Ok(WindowsVersion::Windows8)
    } else if is_windows7_or_greater() {
        Ok(WindowsVersion::Windows7)
    } else if is_windows_vista_or_greater() {
        Ok(WindowsVersion::WindowsVista)
    } else if is_windows_xpor_greater() {
        Ok(WindowsVersion::WindowsXp)
    } else {
        Err(BootstrapError::OsVersionNotFound)
    }
}

#[derive(Debug, Clone, PartialEq, PartialOrd)]
pub enum WindowsVersion {
    WindowsXp = 0,
    WindowsVista = 1,
    Windows7 = 2,
    Windows8 = 3,
    Windows81 = 4,
    Windows10 = 5,
}
impl Default for WindowsVersion {
    fn default() -> Self {
        WindowsVersion::WindowsXp
    }
}

pub fn is_run_as_admin() -> bool {
    use std::ptr;
    use winapi::um::securitybaseapi::{AllocateAndInitializeSid, CheckTokenMembership, FreeSid};
    use winapi::um::winnt::{
        DOMAIN_ALIAS_RID_ADMINS, SECURITY_BUILTIN_DOMAIN_RID, SECURITY_NT_AUTHORITY,
        SID_IDENTIFIER_AUTHORITY,
    };
    unsafe {
        let mut administrator_group = ptr::null_mut();
        if AllocateAndInitializeSid(
            &mut SID_IDENTIFIER_AUTHORITY {
                Value: SECURITY_NT_AUTHORITY,
            },
            2,
            SECURITY_BUILTIN_DOMAIN_RID,
            DOMAIN_ALIAS_RID_ADMINS,
            0,
            0,
            0,
            0,
            0,
            0,
            &mut administrator_group,
        ) == 0
        {
            FreeSid(administrator_group);
            return false;
        }
        let mut is_member = 0;
        if CheckTokenMembership(ptr::null_mut() as _, administrator_group, &mut is_member) == 0 {
            FreeSid(administrator_group);
        }
        is_member != 0
    }
}

pub fn is_elevated() -> bool {
    use std::mem;
    use winapi::shared::minwindef::{DWORD, LPVOID};
    use winapi::um::processthreadsapi::{GetCurrentProcess, OpenProcessToken};
    use winapi::um::securitybaseapi::GetTokenInformation;
    use winapi::um::winnt::{TokenElevation, HANDLE, TOKEN_ELEVATION, TOKEN_QUERY};
    unsafe {
        let mut current_token_ptr: HANDLE = mem::zeroed();
        let mut token_elevation: TOKEN_ELEVATION = mem::zeroed();
        let token_elevation_type_ptr: *mut TOKEN_ELEVATION = &mut token_elevation;
        let mut size: DWORD = 0;

        let result = OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut current_token_ptr);

        if result != 0 {
            let result = GetTokenInformation(
                current_token_ptr,
                TokenElevation,
                token_elevation_type_ptr as LPVOID,
                mem::size_of::<winapi::um::winnt::TOKEN_ELEVATION_TYPE>() as u32,
                &mut size,
            );
            if result != 0 {
                return token_elevation.TokenIsElevated != 0;
            }
        }
    }
    false
}
