use chrono::{DateTime, Utc};
use derive_more::Display;
use std::mem::discriminant;
use std::sync::Arc;
use utoipa::ToSchema;

/// XLS data retrieval errors.
#[derive(Clone, Debug, ToSchema, Display)]
pub enum FetchError {
    /// File url is not set.
    #[display("The link to the timetable was not provided earlier.")]
    NoUrlProvided,

    /// Unknown error.
    #[display("An unknown error occurred while downloading the file.")]
    #[schema(value_type = String)]
    Unknown(Arc<reqwest::Error>),

    /// Server returned a status code different from 200.
    #[display("Server returned a status code {_0}.")]
    BadStatusCode(u16),

    /// The url leads to a file of a different type.
    #[display("The link leads to a file of type '{_0}'.")]
    BadContentType(String),

    /// Server doesn't return expected headers.
    #[display("Server doesn't return expected header(s) '{_0}'.")]
    BadHeaders(String),
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
