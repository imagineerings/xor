use std::path::{Path, PathBuf};

use anyhow::{Context as _, Result, anyhow};
use futures::StreamExt as _;
use reqwest::header::{HeaderMap, HeaderValue, RANGE};
use serde::Serialize;
use thiserror::Error;
use url::Url;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DownloadRequest {
    pub url: Url,
    pub destination: PathBuf,
    pub resume: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub struct DownloadProgress {
    pub downloaded_bytes: u64,
    pub total_bytes: Option<u64>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DownloadResult {
    pub destination: PathBuf,
    pub downloaded_bytes: u64,
}

#[derive(Debug, Error)]
pub enum DownloadError {
    #[error("download URL scheme is not supported: {0}")]
    UnsupportedScheme(String),
}

pub struct DownloadManager {
    client: reqwest::Client,
}

impl Default for DownloadManager {
    fn default() -> Self {
        Self::new()
    }
}

impl DownloadManager {
    pub fn new() -> Self {
        Self {
            client: reqwest::Client::new(),
        }
    }

    pub async fn download(
        &self,
        request: DownloadRequest,
        mut on_progress: impl FnMut(DownloadProgress),
    ) -> Result<DownloadResult> {
        match request.url.scheme() {
            "http" | "https" => self.download_http(request, &mut on_progress).await,
            "file" => self.download_file(request, &mut on_progress).await,
            scheme => Err(DownloadError::UnsupportedScheme(scheme.to_string()).into()),
        }
    }

    async fn download_http(
        &self,
        request: DownloadRequest,
        on_progress: &mut dyn FnMut(DownloadProgress),
    ) -> Result<DownloadResult> {
        let existing_bytes = if request.resume {
            existing_file_len(&request.destination).await?
        } else {
            0
        };

        let mut headers = HeaderMap::new();
        if existing_bytes > 0 {
            headers.insert(
                RANGE,
                HeaderValue::from_str(&format!("bytes={existing_bytes}-"))
                    .context("failed to build range header")?,
            );
        }

        let response = self
            .client
            .get(request.url.clone())
            .headers(headers)
            .send()
            .await?
            .error_for_status()?;

        let total_bytes = response
            .content_length()
            .map(|remaining| remaining.saturating_add(existing_bytes));
        let mut downloaded_bytes = existing_bytes;
        let mut file = open_destination(&request.destination, existing_bytes > 0).await?;
        let mut stream = response.bytes_stream();

        on_progress(DownloadProgress {
            downloaded_bytes,
            total_bytes,
        });

        while let Some(chunk) = stream.next().await {
            let chunk = chunk?;
            smol::io::AsyncWriteExt::write_all(&mut file, &chunk).await?;
            downloaded_bytes += chunk.len() as u64;
            on_progress(DownloadProgress {
                downloaded_bytes,
                total_bytes,
            });
        }
        smol::io::AsyncWriteExt::flush(&mut file).await?;

        Ok(DownloadResult {
            destination: request.destination,
            downloaded_bytes,
        })
    }

    async fn download_file(
        &self,
        request: DownloadRequest,
        on_progress: &mut dyn FnMut(DownloadProgress),
    ) -> Result<DownloadResult> {
        let source_path = request
            .url
            .to_file_path()
            .map_err(|_| anyhow!("invalid file URL: {}", request.url))?;
        let source_bytes = smol::fs::read(&source_path).await?;
        let existing_bytes = if request.resume {
            existing_file_len(&request.destination).await?
        } else {
            0
        };
        let start = existing_bytes.min(source_bytes.len() as u64) as usize;
        let mut file = open_destination(&request.destination, start > 0).await?;

        on_progress(DownloadProgress {
            downloaded_bytes: start as u64,
            total_bytes: Some(source_bytes.len() as u64),
        });
        smol::io::AsyncWriteExt::write_all(&mut file, &source_bytes[start..]).await?;
        smol::io::AsyncWriteExt::flush(&mut file).await?;
        on_progress(DownloadProgress {
            downloaded_bytes: source_bytes.len() as u64,
            total_bytes: Some(source_bytes.len() as u64),
        });

        Ok(DownloadResult {
            destination: request.destination,
            downloaded_bytes: source_bytes.len() as u64,
        })
    }
}

async fn existing_file_len(path: &Path) -> Result<u64> {
    match smol::fs::metadata(path).await {
        Ok(metadata) => Ok(metadata.len()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(0),
        Err(error) => Err(error.into()),
    }
}

async fn open_destination(path: &Path, append: bool) -> Result<smol::fs::File> {
    if let Some(parent) = path.parent() {
        smol::fs::create_dir_all(parent).await?;
    }

    Ok(smol::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .append(append)
        .truncate(!append)
        .open(path)
        .await?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn downloads_file_urls_with_progress() {
        async_io::block_on(async {
            let temp_dir = tempfile::tempdir().expect("create temp dir");
            let source_path = temp_dir.path().join("source.txt");
            let destination = temp_dir.path().join("out").join("destination.txt");
            smol::fs::write(&source_path, "hello")
                .await
                .expect("write source");
            let url = Url::from_file_path(&source_path).expect("file URL");
            let mut progress = Vec::new();

            let result = DownloadManager::new()
                .download(
                    DownloadRequest {
                        url,
                        destination: destination.clone(),
                        resume: false,
                    },
                    |event| progress.push(event),
                )
                .await
                .expect("download file");

            assert_eq!(result.downloaded_bytes, 5);
            assert_eq!(
                smol::fs::read_to_string(destination).await.unwrap(),
                "hello"
            );
            assert_eq!(progress.last().unwrap().downloaded_bytes, 5);
        });
    }

    #[test]
    fn resumes_file_urls() {
        async_io::block_on(async {
            let temp_dir = tempfile::tempdir().expect("create temp dir");
            let source_path = temp_dir.path().join("source.txt");
            let destination = temp_dir.path().join("destination.txt");
            smol::fs::write(&source_path, "hello world")
                .await
                .expect("write source");
            smol::fs::write(&destination, "hello")
                .await
                .expect("write partial");
            let url = Url::from_file_path(&source_path).expect("file URL");

            DownloadManager::new()
                .download(
                    DownloadRequest {
                        url,
                        destination: destination.clone(),
                        resume: true,
                    },
                    |_| {},
                )
                .await
                .expect("resume download");

            assert_eq!(
                smol::fs::read_to_string(destination).await.unwrap(),
                "hello world"
            );
        });
    }
}
