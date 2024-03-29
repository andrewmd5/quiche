use crate::io::ico::IcoError;
use crate::updater::{ReleaseBranch, UpdateType};
use std::fmt;

#[derive(Debug)]
pub enum BootstrapError {
    RecipeBakeFailure(String),
    RecipeStageFailure(String),
    ElevationRequired,
    ServiceConnectionFailure,
    ServiceOpenFailure,
    ServiceQueryFailed,
    ServiceInstalled(String),
    ServiceMissing(String),
    DismFailed(String),
    ArchitectureUnsupported,
    WindowsVersionUnsupported,
    NeedWindowsMediaPack(String),
    NeedDotNetFramework,
    RegistryKeyNotFound(String),
    RegistryValueNotFound(String),
    HttpFailed(String),
    LocalVersionMissing,
    InstallPathMissing,
    ReleaseLookupFailed(String),
    VersionCheckFailed(String, String),
    TomlParseFailure(String, String),
    BootstrapperExist,
    SignatureMismatch,
    RemoteFileMissing(String),
    RemoteFileEmpty(String),
    InstallationFailed(String),
    RequestError(hyper::Error),
    IOError(std::io::Error),
    WebView(String),
    ResourceLoadError(String),
    IcoError(String),
    UninstallEntryMissing,
    UnableToSetRegKey(String),
    OsVersionNotFound,
    NewSidFailed,
    SidUpdateFailed,
    ServiceInstallFailed,
}

#[allow(non_snake_case)]
impl fmt::Display for BootstrapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            BootstrapError::RecipeBakeFailure(ref s) => write!(f, "Unable to complete release build due to baking issue: {0}", s),
            BootstrapError::RecipeStageFailure(ref s) => write!(f, "Unable to complete release build due to staging issue: {0}", s),
            BootstrapError::ElevationRequired => write!(f, "Please run the Rainway Boostrapper as Administrator."),
            BootstrapError::DismFailed(ref s) => write!(f, "DISM failed to launch: {0}", s),
            BootstrapError::ServiceConnectionFailure => write!(f, "Failed to connect to the system service manager."),
            BootstrapError::ServiceOpenFailure => write!(f, "Failed to open target service for interaction."),
            BootstrapError::ServiceQueryFailed => write!(f, "Failed to query status of the service."),
            BootstrapError::ServiceInstalled(ref s) => write!(f, "Unable to install the system service {0} as it already exist.", s),
            BootstrapError::ServiceMissing(ref s) => write!(f, "Unable to start the system service {0} because it is not installed.", s),
            BootstrapError::ArchitectureUnsupported => write!(f, "Rainway is currently only supported by x64 operating systems."),
            BootstrapError::WindowsVersionUnsupported => write!(f, "Rainway is currently only supported on Windows 10 and Windows Server 2016+."),
            BootstrapError::NeedWindowsMediaPack(ref s) => write!(f, "A required video codec is missing from your system. Please inslaunchtall the Windows Media Pack for {}.\n\nPress \"Ok\" to open the codec download page.", s),
            BootstrapError::NeedDotNetFramework => write!(f, ".NET Framework 4.7.2 is missing from your computer and is required to install Rainway.\n\nPress \"Ok\" to open the .NET Framework download page."),
            BootstrapError::RegistryKeyNotFound(ref s) => write!(f, "An error occured accessing Windows Registry key: {}.", s),
            BootstrapError::RegistryValueNotFound(ref s) => write!(f, "An error occured accessing Windows Registry value: {}.", s),
            BootstrapError::HttpFailed(ref s) => write!(f, "{}", s),
            BootstrapError::VersionCheckFailed(ref rv, ref lv) => write!(f, "Unable to compare remote version ({}) to installed version ({}).", rv, lv),
            BootstrapError::TomlParseFailure(ref s, ref e) => write!(f, "An exception was encountered parsing a remote file located at {} due to {}", s, e),
            BootstrapError::SignatureMismatch => write!(f, "We were unable to validate the updates integrity. Please exit and try again."),
            BootstrapError::RemoteFileMissing(ref s) => write!(f, "The remote file requested ({}) is not present at the address provided.", s),
            BootstrapError::RemoteFileEmpty(ref s) => write!(f, "The remote file requested ({}) is has a zero byte length.", s),
            BootstrapError::InstallationFailed(ref s) => write!(f, "An error occured installing the latest update: {0}", s),
            BootstrapError::RequestError(ref e) => write!(f, "An unknown network issue was encountered accessing {0}", e),
            BootstrapError::IOError(ref e) => write!(f, "An unknown issue was encountered: {0}", e),
            BootstrapError::BootstrapperExist => write!(f, "Another instance of the Rainway Bootstrapper is already running."),
            BootstrapError::WebView(ref e) => write!(f, "An unknown UI issue was encountered: {0}", e),
            BootstrapError::LocalVersionMissing => write!(f, "Unable to locate the version of the currently intalled branch."),
            BootstrapError::InstallPathMissing => write!(f, "Unable to locate the installation path of the currently intalled branch."),
            BootstrapError::ReleaseLookupFailed(ref e) => write!(f, "Looks like something went wrong. We were unable to determine the latest Rainway release. Please exit and try again. \n\n {0}", e),
            BootstrapError::ResourceLoadError(ref e) => write!(f, "Failed to load application resource. {0}", e),
            BootstrapError::IcoError(ref e) => write!(f, "{0}", e),
            BootstrapError::UninstallEntryMissing => write!(f, "No Uninstall key entry was present for {0}.", env!("UNINSTALL_KEY")),
            BootstrapError::UnableToSetRegKey(ref e) => write!(f, "Unable to set regkey {0}", e),
            BootstrapError::OsVersionNotFound => write!(f, "Unable to determine a specified operating system version attribute."),
            BootstrapError::NewSidFailed => write!(f, "Failed to create new SID"),
            BootstrapError::SidUpdateFailed => write!(f, "Failed to add SID"),
            BootstrapError::ServiceInstallFailed => write!(f, "Failed to install service"),
        }
    }
}

impl From<String> for ReleaseBranch {
    fn from(branch: String) -> Self {
        match branch.to_lowercase().trim() {
            "stable" => ReleaseBranch::Stable,
            "nightly" => ReleaseBranch::Nightly,
            "beta" => ReleaseBranch::Beta,
            _ => ReleaseBranch::Stable,
        }
    }
}

impl From<&str> for ReleaseBranch {
    fn from(branch: &str) -> Self {
        ReleaseBranch::from(branch.to_string())
    }
}

impl Default for ReleaseBranch {
    fn default() -> ReleaseBranch {
        ReleaseBranch::Stable
    }
}

impl From<hyper::Error> for BootstrapError {
    fn from(error: hyper::Error) -> Self {
        BootstrapError::RequestError(error)
    }
}

impl From<std::io::Error> for BootstrapError {
    fn from(error: std::io::Error) -> Self {
        BootstrapError::IOError(error)
    }
}

impl From<IcoError> for BootstrapError {
    fn from(error: IcoError) -> Self {
        BootstrapError::IcoError(error.to_string())
    }
}

impl From<std::str::Utf8Error> for BootstrapError {
    fn from(_error: std::str::Utf8Error) -> Self {
        BootstrapError::WebView("Unable to parse UTF8 source.".to_string())
    }
}

impl Default for UpdateType {
    fn default() -> UpdateType {
        UpdateType::Install
    }
}

impl fmt::Display for UpdateType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}
impl Copy for UpdateType {}

impl Clone for UpdateType {
    fn clone(&self) -> UpdateType {
        *self
    }
}

impl fmt::Display for ReleaseBranch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// checks if the executable has been compiled against a x64 target.
pub fn is_compiled_for_64_bit() -> bool {
    cfg!(target_pointer_width = "64")
}
