use crate::updater::{ReleaseBranch, UpdateState, UpdateType};
use std::fmt;

#[derive(Debug)]
pub enum BootstrapError {
    ElevationRequired,
    ServiceConnectionFailure,
    ServiceOpenFailure,
    ServiceQueryFailed,
    ServiceMissing(String),
    DismFailed(String),
    ArchitectureUnsupported,
    WindowsVersionUnsupported,
    NeedWindowsMediaPack(String),
    AlreadyInstalled,
    RegistryKeyNotFound(String),
    RegistryValueNotFound(String),
    HttpFailed(u16, String),
    ReleaseLookupFailed,
    VersionCheckFailed(String, String),
    TomlParseFailure(String, String),
    JsonParseFailure,
    BootstrapperExist,
    SignatureMismatch,
    RemoteFileMissing(String),
    RemoteFileEmpty(String),
    InstallationFailed(String),
    RequestError(reqwest::Error),
    IOError(std::io::Error),
    WebView(web_view::Error),
}

#[allow(non_snake_case)]
impl fmt::Display for BootstrapError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match *self {
            BootstrapError::ElevationRequired => write!(f, "Please run the Rainway Boostrapper as Administrator."),
            BootstrapError::DismFailed(ref s) => write!(f, "DISM failed to launch: {0}", s),
            BootstrapError::ServiceConnectionFailure => write!(f, "Failed to connect to the system service manager."),
            BootstrapError::ServiceOpenFailure => write!(f, "Failed to open target service for interaction."),
            BootstrapError::ServiceQueryFailed => write!(f, "Failed to query status of the service."),
            BootstrapError::ServiceMissing(ref s) => write!(f, "Unable to start the system service {0} because it is not installed.", s),
            BootstrapError::ArchitectureUnsupported => write!(f, "Rainway is currently only supported by x64 operating systems."),
            BootstrapError::WindowsVersionUnsupported => write!(f, "Rainway is currently only supported on Windows 10 and Windows Server 2016+."),
            BootstrapError::NeedWindowsMediaPack(ref s) => write!(f, "A required video codec is missing from your system. Please install the Windows Media Pack for {}.\n\nPress \"Ok\" to open the codec download page.", s),
            BootstrapError::AlreadyInstalled => write!(f, "Rainway is already installed on this computer."),
            BootstrapError::RegistryKeyNotFound(ref s) => write!(f, "An error occured accessing Windows Registry key: {}.", s),
            BootstrapError::RegistryValueNotFound(ref s) => write!(f, "An error occured accessing Windows Registry value: {}.", s),
            BootstrapError::HttpFailed(ref c, ref s) => write!(f, "Network connection issue occured accessing {}: {}.", s, c),
            BootstrapError::VersionCheckFailed(ref rv, ref lv) => write!(f, "Unable to compare remote version ({}) to installed version ({}).", rv, lv),
            BootstrapError::TomlParseFailure(ref s, ref e) => write!(f, "An exception was encountered parsing a remote file {} due to: {}", s, e),
            BootstrapError::JsonParseFailure => write!(f, "We're having trouble determining the current version of Rainway. Please exit and try again."),
            BootstrapError::SignatureMismatch => write!(f, "We were unable to validate the updates integrity. Please exit and try again."),
            BootstrapError::RemoteFileMissing(ref s) => write!(f, "The remote file requested ({}) is not present at the address provided.", s),
            BootstrapError::RemoteFileEmpty(ref s) => write!(f, "The remote file requested ({}) is has a zero byte length.", s),
            BootstrapError::InstallationFailed(ref s) => write!(f, "An error occured installing the latest update: {0}", s),
            BootstrapError::RequestError(ref e) => write!(f, "An unknown network issue was encountered: {0}", e),
            BootstrapError::IOError(ref e) => write!(f, "An unknown issue was encountered: {0}", e),
            BootstrapError::BootstrapperExist => write!(f, "Another instance of the Rainway Bootstrapper is already running."),
            BootstrapError::WebView(ref e) => write!(f, "An unknown UI issue was encountered: {0}", e),
            BootstrapError::ReleaseLookupFailed => write!(f, "Looks like something went wrong. We were unable to determine the latest Rainway release. Please exit and try again."),
        }
    }
}

impl From<String> for ReleaseBranch {
    fn from(branch: String) -> Self {
        match branch.to_lowercase().trim() {
            "stable" => ReleaseBranch::Stable,
            "nightly" => ReleaseBranch::Nightly,
            "beta" => ReleaseBranch::Beta,
            _ => unimplemented!() // TODO
        }
    }
}

impl From<reqwest::Error> for BootstrapError {
    fn from(error: reqwest::Error) -> Self {
        BootstrapError::RequestError(error)
    }
}

impl From<std::io::Error> for BootstrapError {
    fn from(error: std::io::Error) -> Self {
        BootstrapError::IOError(error)
    }
}

impl From<web_view::Error> for BootstrapError {
    fn from(error: web_view::Error) -> Self {
        BootstrapError::WebView(error)
    }
}

impl Default for UpdateState {
    fn default() -> UpdateState {
        UpdateState::None
    }
}

impl Default for UpdateType {
    fn default() -> UpdateType {
        UpdateType::Install
    }
}

impl fmt::Display for UpdateState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
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

impl Copy for UpdateState {}

impl Clone for UpdateState {
    fn clone(&self) -> UpdateState {
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
