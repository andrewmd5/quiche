use crate::etc::constants::BootstrapError;
use hyper::body::HttpBody as _;
use hyper::Client;
use hyper::{header::HeaderValue, Body, Request};
use hyper_tls::HttpsConnector;
use serde::de::DeserializeOwned;
use std::path::PathBuf;
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
        let mut https = HttpsConnector::new();
        https.https_only(true);
        let client = Client::builder().build::<_, hyper::Body>(https);

        let request = match Request::get(url).body(Body::empty()) {
            Ok(b) => b,
            Err(e) => return Err(BootstrapError::HttpFailed(e.to_string())),
        };

        let mut response = match client.request(request).await {
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

        let mut buffer: Vec<u8> = Vec::new();
        while let Some(chunk) = response.body_mut().data().await {
            buffer.append(&mut chunk?.to_vec());
        }
        match toml::from_slice(&buffer) {
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
    let mut https = HttpsConnector::new();
    https.https_only(true);
    let client = Client::builder().build::<_, hyper::Body>(https);

    let head_request = match Request::head(url).body(Body::empty()) {
        Ok(b) => b,
        Err(e) => return Err(BootstrapError::HttpFailed(e.to_string())),
    };

    let head_response = match client.request(head_request).await {
        Ok(r) => r,
        Err(e) => return Err(BootstrapError::HttpFailed(e.to_string())),
    };

    if !head_response.status().is_success() {
        log::error!("unable to download {} as the remote file is missing.", url);
        return Err(BootstrapError::RemoteFileMissing(url.to_string()));
    }

    let total_size = head_response
        .headers()
        .get(hyper::header::CONTENT_LENGTH)
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

    let download_request = match Request::get(url).body(Body::empty()) {
        Ok(b) => b,
        Err(e) => return Err(BootstrapError::HttpFailed(e.to_string())),
    };
    let mut download_response = match client.request(download_request).await {
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
    while let Some(chunk) = download_response.body_mut().data().await {
        let read_bytes = temp_file.write(&chunk?).await?;
        total_downloaded_bytes += read_bytes as u64;
        callback(total_size, total_downloaded_bytes);
    }

    log::info!("downloaded {} bytes.", total_downloaded_bytes);
    temp_file.flush().await?;
    drop(temp_file);
    Ok(total_downloaded_bytes == total_size)
}

// Blocking post a body to a url
pub fn post(
    url: &str,
    json: String,
    custom_headers: Option<std::collections::HashMap<&'static str, &'static str>>,
) -> Result<hyper::StatusCode, BootstrapError> {
    use tokio::runtime::Runtime;
    let mut runtime = match Runtime::new() {
        Ok(rt) => rt,
        Err(e) => return Err(BootstrapError::from(e)),
    };

    let results = runtime.block_on(async {
        let mut https = HttpsConnector::new();
        https.https_only(true);
        let client = Client::builder().build::<_, hyper::Body>(https);

        let mut builder = Request::builder().method("POST").uri(url);
        if let Some(headers) = builder.headers_mut() {
            if let Some(header_map) = custom_headers {
                for (k, v) in header_map {
                    headers.insert(k, HeaderValue::from_str(v).unwrap());
                }
            }
        }

        let req = match builder.body(Body::from(json.as_bytes().to_owned())) {
            Ok(r) => r,
            Err(e) => return Err(BootstrapError::HttpFailed(e.to_string())),
        };
        let mut response = match client.request(req).await {
            Ok(g) => g,
            Err(e) => return Err(BootstrapError::HttpFailed(e.to_string())),
        };

        Ok(response.status())
    });

    drop(runtime);

    results
}
