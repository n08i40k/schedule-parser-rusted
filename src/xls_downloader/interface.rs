use chrono::{DateTime, Utc};
use std::mem::discriminant;

/// XLS data retrieval errors.
#[derive(Debug)]
pub enum FetchError {
    /// File url is not set.
    NoUrlProvided,

    /// Unknown error.
    Unknown(reqwest::Error),

    /// Server returned a status code different from 200.
    BadStatusCode,

    /// The url leads to a file of a different type.
    BadContentType,

    /// Server doesn't return expected headers.
    BadHeaders,
}

impl PartialEq for FetchError {
    fn eq(&self, other: &Self) -> bool {
        discriminant(self) == discriminant(other)
    }
}

/// Result of XLS data retrieval.
pub struct FetchOk {
    /// ETag object.
    pub etag: String,

    /// File upload date.
    pub uploaded_at: DateTime<Utc>,

    /// Date data received.
    pub requested_at: DateTime<Utc>,

    /// File data.
    pub data: Option<Vec<u8>>,
}

impl FetchOk {
    /// Result without file content.
    pub fn head(etag: String, uploaded_at: DateTime<Utc>) -> Self {
        FetchOk {
            etag,
            uploaded_at,
            requested_at: Utc::now(),
            data: None,
        }
    }

    /// Full result.
    pub fn get(etag: String, uploaded_at: DateTime<Utc>, data: Vec<u8>) -> Self {
        FetchOk {
            etag,
            uploaded_at,
            requested_at: Utc::now(),
            data: Some(data),
        }
    }
}

pub type FetchResult = Result<FetchOk, FetchError>;

pub trait XLSDownloader {
    /// Get data about the file, and optionally its content.
    async fn fetch(&self, head: bool) -> FetchResult;

    /// Setting the file link.
    async fn set_url(&mut self, url: String) -> FetchResult;
}
