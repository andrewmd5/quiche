use crate::etc::constants::BootstrapError;
use crate::updater::{ActiveUpdate, UpdateState};
use reqwest::{header, Client};
use serde::de::DeserializeOwned;
use serde_json;
use std::fs::OpenOptions;
use std::io::{self, copy, Read};
use std::path::PathBuf;

struct DownloadProgress<R> {
    inner: R,
    progress: std::sync::Arc<std::sync::RwLock<ActiveUpdate>>,
}

impl<R: Read> Read for DownloadProgress<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buf).map(|n| {
            let mut writer = self.progress.write().unwrap();
            if writer.state == UpdateState::None {
                writer.state = UpdateState::Downloading;
            }
            writer.downloaded_bytes += n as u64;
            drop(writer);
            n
        })
    }
}

/// Downloads a remote TOML string and deseralizes it into a provided <T> generic.
pub fn download_toml<T>(url: &str) -> Result<T, BootstrapError>
where
    T: DeserializeOwned,
{
    let mut response = reqwest::get(url)?;
    if !response.status().is_success() {
        return Err(BootstrapError::HttpFailed(
            response.status().as_u16(),
            url.to_string(),
        ));
    }
    let toml = response.text()?;
    match toml::from_str(&toml) {
        Err(_e) => return Err(BootstrapError::TomlParseFailure),
        Ok(model) => return Ok(model),
    };
}

/// Downloads a remote JSON string and deseralizes it into a provided <T> generic.
pub fn download_json<T>(url: &str) -> Result<T, BootstrapError>
where
    T: DeserializeOwned,
{
    let mut response = reqwest::get(url)?;
    if !response.status().is_success() {
        return Err(BootstrapError::HttpFailed(
            response.status().as_u16(),
            url.to_string(),
        ));
    }
    let json = response.text()?;
    match serde_json::from_str(&json) {
        Err(_e) => return Err(BootstrapError::JsonParseFailure),
        Ok(model) => return Ok(model),
    };
}

/// Downloads a file from a remote URL and saves it to the output path supplied.
pub fn download_file(
    r: std::sync::Arc<std::sync::RwLock<ActiveUpdate>>,
    url: &str,
    path: &PathBuf,
) -> Result<bool, BootstrapError> {
    let client = Client::new();
    let head_response = client.head(url).send()?;
    if !head_response.status().is_success() {
        return Err(BootstrapError::InstallerDownloadFailed);
    }
    let total_size = head_response
        .headers()
        .get(header::CONTENT_LENGTH)
        .and_then(|ct_len| ct_len.to_str().ok())
        .and_then(|ct_len| ct_len.parse().ok())
        .unwrap_or(0);
    if total_size <= 0 {
        return Err(BootstrapError::InstallerDownloadFailed);
    }

    let mut temp_file = OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(path)?;

    let request = client.get(url);

    let mut writer = r.write().unwrap();
    writer.total_bytes = total_size;
    drop(writer);

    let get_response = request.send()?;
    let mut source = DownloadProgress {
        progress: r,
        inner: get_response,
    };
    match copy(&mut source, &mut temp_file) {
        Err(e) => return Err(BootstrapError::IOError(e)),
        Ok(r) => return Ok(r == total_size),
    };
}
