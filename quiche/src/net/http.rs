use crate::etc::constants::BootstrapError;
use crate::updater::{UpdateDownloadProgress, UpdateState};
use reqwest::{header, Client};
use serde::de::DeserializeOwned;
use std::fs::OpenOptions;
use std::io::{self, copy, Read};
use std::path::PathBuf;
use std::time::Duration;

struct DownloadProgress<R> {
    inner: R,
    progress: std::sync::Arc<std::sync::RwLock<UpdateDownloadProgress>>,
}

impl<R: Read> Read for DownloadProgress<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.inner.read(buf).map(|n| {
            let mut writer = self.progress.write().unwrap();
            writer.downloaded_bytes += n as u64;
            if writer.state == UpdateState::None {
                writer.state = UpdateState::Downloading;
            }
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
    let mut response = match reqwest::get(url) {
        Ok(r) => r,
        Err(e) => return Err(BootstrapError::HttpFailed(e.to_string())),
    };
    if !response.status().is_success() {
        return Err(BootstrapError::HttpFailed(format!(
            "[STATUS] {} could not reach {}",
            response.status().as_u16(),
            url.to_string()
        )));
    }
    let toml = response.text()?;
    match toml::from_str(&toml) {
        Err(e) => {
            return Err(BootstrapError::TomlParseFailure(
                url.to_string(),
                e.to_string(),
            ))
        }
        Ok(model) => return Ok(model),
    };
}

/// Downloads a file from a remote URL and saves it to the output path supplied.
/// We must explicitly handle all exceptions in here to drop the writer
/// or we risk deadlocking the thread.
pub fn download_file(
    r: std::sync::Arc<std::sync::RwLock<UpdateDownloadProgress>>,
    url: &str,
    path: &PathBuf,
) -> Result<bool, BootstrapError> {
    let mut writer = r.write().unwrap();

    let client = match Client::builder().timeout(Duration::from_secs(10)).build() {
        Ok(c) => c,
        Err(e) => {
            writer.faulted = true;
            drop(writer);
            log::error!("failed to configure client for {}.", url);
            return Err(BootstrapError::HttpFailed(e.to_string()));
        }
    };

    let head_response = match client.head(url).send() {
        Ok(r) => r,
        Err(e) => {
            writer.faulted = true;
            drop(writer);
            log::error!("failed to send HEAD request to {}.", url);
            return Err(BootstrapError::HttpFailed(e.to_string()));
        }
    };

    if !head_response.status().is_success() {
        writer.faulted = true;
        drop(writer);
        log::error!("unable to download {} as the remote file is missing.", url);
        return Err(BootstrapError::RemoteFileMissing(url.to_string()));
    }

    let total_size = head_response
        .headers()
        .get(header::CONTENT_LENGTH)
        .and_then(|ct_len| ct_len.to_str().ok())
        .and_then(|ct_len| ct_len.parse().ok())
        .unwrap_or(0);

    if total_size <= 0 {
        writer.faulted = true;
        drop(writer);
        log::error!("unable to download {} as the remote file is empty.", url);
        return Err(BootstrapError::RemoteFileEmpty(url.to_string()));
    }
    writer.total_bytes = total_size;

    let mut temp_file = match OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(path)
    {
        Ok(f) => f,
        Err(e) => {
            writer.faulted = true;
            drop(writer);
            log::error!("failed to create temporary file.");
            return Err(BootstrapError::HttpFailed(e.to_string()));
        }
    };

    let request = client.get(url);
    let get_response = match request.send() {
        Ok(g) => g,
        Err(e) => {
            writer.faulted = true;
            drop(writer);
            log::error!("failed to get response from {}", url);
            return Err(BootstrapError::HttpFailed(e.to_string()));
        }
    };
    //now we're safe
    drop(writer);
    let mut source = DownloadProgress {
        progress: r,
        inner: get_response,
    };
    log::info!(
        "starting download of {} ({} bytes) to {}.",
        url,
        total_size,
        path.display()
    );

    let r = match copy(&mut source, &mut temp_file) {
        Err(e) => return Err(BootstrapError::IOError(e)),
        Ok(r) => r,
    };

    drop(temp_file);
    log::info!("downloaded {} bytes.", r);
    Ok(r == total_size)
}
