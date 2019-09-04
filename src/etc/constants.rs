use crate::updater::{UpdateState, ReleaseBranch};
use std::fmt;

#[derive(Debug)]
pub enum BootstrapError {
    ElevationRequired,
    DismFailed(String),
    ArchitectureUnsupported,
    WindowsVersionUnsupported,
    NeedWindowsMediaPack(String),
    AlreadyInstalled,
    RegistryKeyNotFound(String),
    RegistryValueNotFound(String),
    HttpFailed(u16, String),
    ReleaseLookupFailed,
    TomlParseFailure,
    JsonParseFailure,
    BootstrapperExist,
    SignatureMismatch,
    InstallerDownloadFailed,
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
            BootstrapError::ArchitectureUnsupported => write!(f, "Rainway is currently only supported by x64 operating systems."),
            BootstrapError::WindowsVersionUnsupported => write!(f, "Rainway is currently only supported on Windows 10 and Windows Server 2016+."),
            BootstrapError::NeedWindowsMediaPack(ref s) => write!(f, "A required video codec is missing from your system. Please install the Windows Media Pack for {}.\n\nPress \"Ok\" to open the codec download page.", s),
            BootstrapError::AlreadyInstalled => write!(f, "Rainway is already installed on this computer."),
            BootstrapError::RegistryKeyNotFound(ref s) => write!(f, "An error occured accessing Windows Registry key: {}.", s),
            BootstrapError::RegistryValueNotFound(ref s) => write!(f, "An error occured accessing Windows Registry value: {}.", s),
            BootstrapError::HttpFailed(ref c, ref s) => write!(f, "Network connection issue occured accessing {}: {}.", s, c),
            BootstrapError::TomlParseFailure => write!(f, "We're having trouble determining the current version of Rainway. Please exit and try again."),
            BootstrapError::JsonParseFailure => write!(f, "We're having trouble determining the current version of Rainway. Please exit and try again."),
            BootstrapError::SignatureMismatch => write!(f, "We were unable to validate the downloaded update. Please exit and try again."),
            BootstrapError::InstallerDownloadFailed => write!(f, "We were unable to downloaded the latest Rainway update. Please exit and try again."),
            BootstrapError::InstallationFailed(ref s) => write!(f, "An error occured installing the latest update: {0}", s),
            BootstrapError::RequestError(ref e) => write!(f, "An unknown network issue was encountered: {0}", e),
            BootstrapError::IOError(ref e) => write!(f, "An unknown issue was encountered: {0}", e),
            BootstrapError::BootstrapperExist => write!(f, "Another instance of the Rainway Bootstrapper is already running."),
            BootstrapError::WebView(ref e) => write!(f, "An unknown UI issue was encountered: {0}", e),
            BootstrapError::ReleaseLookupFailed => write!(f, "Looks like something went wrong. We were unable to determine the latest Rainway release. Please exit and try again."),
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

impl fmt::Display for UpdateState {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl fmt::Display for  ReleaseBranch {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?}", self)
    }
}

/// checks if the executable has been compiled against a x64 target.
pub fn is_compiled_for_64_bit() -> bool {
    cfg!(target_pointer_width = "64")
}