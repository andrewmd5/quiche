use std::error::Error;
use std::fmt;
use std::fmt::{Display, Formatter, Result};

#[derive(Debug)]
pub enum BootstrapError {
    ElevationRequired,
    DismFailed(String),
    ArchitectureUnsupported,
    WindowsVersionUnsupported,
    NeedWindowsMediaPack,
    AlreadyInstalled,
    RegistryKeyNotFound(String),
    RegistryValueNotFound(String),
    HttpFailed(u16, String),
    JsonParseFailure,
    SignatureMismatch,
    InstallerDownloadFailed,
    InstallationFailed(String),
    RequestError(reqwest::Error),
    IOError(std::io::Error),
}

impl fmt::Display for BootstrapError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self {
            ElevationRequired => write!(f, ": "),
            _ => write!(f, "{}", 2),
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
