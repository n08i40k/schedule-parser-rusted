use chrono::{DateTime, Utc};
use derive_more::{Display, Error};
use std::mem::discriminant;
use std::sync::Arc;

mod yandex_disk;

/// XLS data retrieval errors.
#[derive(Clone, Debug, Display, Error)]
pub enum FetchError {
    /// Unknown error.
    #[display("An unknown error occurred while downloading the file.")]
    Reqwest(Arc<reqwest::Error>),

    /// Server returned a status code different from 200.
    #[display("Server returned a status code {status_code}.")]
    BadStatusCode { status_code: u16 },

    /// The folder contains no file matching the schedule name pattern.
    #[display("No schedule file was found in the shared folder.")]
    NoScheduleFile,
}

impl FetchError {
    pub fn unknown(error: Arc<reqwest::Error>) -> Self {
        Self::Reqwest(error)
    }

    pub fn bad_status_code(status_code: u16) -> Self {
        Self::BadStatusCode { status_code }
    }
}

impl PartialEq for FetchError {
    fn eq(&self, other: &Self) -> bool {
        discriminant(self) == discriminant(other)
    }
}

pub type FetchResult<T> = Result<T, FetchError>;

/// Description of the remote schedule file, obtained without downloading its content.
#[derive(Clone, Debug, PartialEq)]
pub struct RemoteFile {
    /// Permanent link to the file, shown to API clients.
    pub url: String,

    /// Link the content is actually downloaded from.
    pub download_url: String,

    /// Content hash, changing whenever the file content changes.
    pub version: String,

    /// Time of the last file modification reported by the remote side.
    pub modified_at: DateTime<Utc>,
}

/// Public Yandex Disk folder the schedule is downloaded from.
#[derive(Clone, Debug)]
pub struct Source {
    pub public_url: String,
}

impl Source {
    pub fn new(public_url: String) -> Self {
        Self { public_url }
    }

    /// Looks up the current schedule file without downloading its content.
    pub async fn probe(&self) -> FetchResult<RemoteFile> {
        yandex_disk::probe(&self.public_url).await
    }

    /// Downloads the content of a previously probed file.
    pub async fn download(&self, file: &RemoteFile) -> FetchResult<Vec<u8>> {
        get(&file.download_url)
            .await?
            .bytes()
            .await
            .map(|bytes| bytes.to_vec())
            .map_err(|error| FetchError::unknown(Arc::new(error)))
    }
}

/// Performs a GET request with a spoofed browser User-Agent and checks the status code.
async fn get(url: &str) -> FetchResult<reqwest::Response> {
    let response = reqwest::Client::new()
        .get(url)
        .header("User-Agent", ua_generator::ua::spoof_chrome_ua())
        .send()
        .await
        .map_err(|error| FetchError::unknown(Arc::new(error)))?;

    if response.status().as_u16() != 200 {
        return Err(FetchError::bad_status_code(response.status().as_u16()));
    }

    Ok(response)
}
