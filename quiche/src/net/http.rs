use crate::etc::constants::BootstrapError;
use reqwest::{header, Client};
use serde::de::DeserializeOwned;
use std::path::PathBuf;
use std::time::Duration;
use tokio::io::AsyncWriteExt;

/// Downloads a remote TOML string and deseralizes it into a provided <T> generic.
pub fn download_toml<T>(url: &str) -> Result<T, BootstrapError>
where
    T: DeserializeOwned,
{
    use tokio::runtime::Runtime;
    let mut runtime = match Runtime::new() {
        Ok(rt) => rt,
        Err(e) => return Err(BootstrapError::from(e)),
    };
    let results = runtime.block_on(async {
        let response = match reqwest::get(url).await {
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
        let toml = response.text().await?;
        match toml::from_str(&toml) {
            Err(e) => {
                return Err(BootstrapError::TomlParseFailure(
                    url.to_string(),
                    e.to_string(),
                ))
            }
            Ok(model) => return Ok(model),
        };
    });
    drop(runtime);
    results
}

/// Downloads a file from a remote URL and saves it to the output path supplied.
/// We must explicitly handle all exceptions in here to drop the writer
/// or we risk deadlocking the thread.
pub async fn download_file<F>(
    callback: F,
    url: &str,
    path: &PathBuf,
) -> Result<bool, BootstrapError>
where
    F: Fn(u64, u64) + Send + Sync + 'static,
{
    let client = match Client::builder()
        .no_trust_dns()
        .connect_timeout(Duration::from_secs(30))
        .build()
    {
        Ok(c) => c,
        Err(e) => return Err(BootstrapError::HttpFailed(e.to_string())),
    };

    let head_response = match client.head(url).send().await {
        Ok(r) => r,
        Err(e) => return Err(BootstrapError::HttpFailed(e.to_string())),
    };

    if !head_response.status().is_success() {
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
        log::error!("unable to download {} as the remote file is empty.", url);
        return Err(BootstrapError::RemoteFileEmpty(url.to_string()));
    }

    let mut temp_file = match tokio::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .open(path)
        .await
    {
        Ok(f) => f,
        Err(e) => return Err(BootstrapError::HttpFailed(e.to_string())),
    };

    let request = client.get(url);
    let mut response = match request.send().await {
        Ok(g) => g,
        Err(e) => return Err(BootstrapError::HttpFailed(e.to_string())),
    };
    //now we're safe
    log::info!(
        "starting download of {} ({} bytes) to {}.",
        url,
        total_size,
        path.display()
    );

    let mut total_downloaded_bytes = 0;
    while let Some(chunk) = response.chunk().await? {
        let read_bytes = match temp_file.write(&chunk).await {
            Ok(r) => r,
            Err(e) => return Err(BootstrapError::from(e)),
        };
        total_downloaded_bytes += read_bytes as u64;
        callback(total_size, total_downloaded_bytes);
    }

    log::info!("downloaded {} bytes.", total_downloaded_bytes);
    temp_file.flush().await?;
    drop(temp_file);
    Ok(total_downloaded_bytes == total_size)
}
