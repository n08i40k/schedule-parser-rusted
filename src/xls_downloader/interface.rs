use chrono::{DateTime, Utc};
use derive_more::{Display, Error};
use std::mem::discriminant;
use std::sync::Arc;
use utoipa::ToSchema;

/// XLS data retrieval errors.
#[derive(Clone, Debug, ToSchema, Display, Error)]
pub enum FetchError {
    /// File url is not set.
    #[display("The link to the timetable was not provided earlier.")]
    NoUrlProvided,

    /// Unknown error.
    #[display("An unknown error occurred while downloading the file.")]
    #[schema(value_type = String)]
    Unknown(Arc<reqwest::Error>),

    /// Server returned a status code different from 200.
    #[display("Server returned a status code {status_code}.")]
    BadStatusCode { status_code: u16 },

    /// The url leads to a file of a different type.
    #[display("The link leads to a file of type '{content_type}'.")]
    BadContentType { content_type: String },

    /// Server doesn't return expected headers.
    #[display("Server doesn't return expected header(s) '{expected_header}'.")]
    BadHeaders { expected_header: String },
}

impl FetchError {
    pub fn unknown(error: Arc<reqwest::Error>) -> Self {
        Self::Unknown(error)
    }

    pub fn bad_status_code(status_code: u16) -> Self {
        Self::BadStatusCode { status_code }
    }

    pub fn bad_content_type(content_type: &str) -> Self {
        Self::BadContentType {
            content_type: content_type.to_string(),
        }
    }

    pub fn bad_headers(expected_header: &str) -> Self {
        Self::BadHeaders {
            expected_header: expected_header.to_string(),
        }
    }
}

impl PartialEq for FetchError {
    fn eq(&self, other: &Self) -> bool {
        discriminant(self) == discriminant(other)
    }
}

/// Result of XLS data retrieval.
pub struct FetchOk {
    /// File upload date.
    pub uploaded_at: DateTime<Utc>,

    /// Date data received.
    pub requested_at: DateTime<Utc>,

    /// File data.
    pub data: Option<Vec<u8>>,
}

impl FetchOk {
    /// Result without file content.
    pub fn head(uploaded_at: DateTime<Utc>) -> Self {
        FetchOk {
            uploaded_at,
            requested_at: Utc::now(),
            data: None,
        }
    }

    /// Full result.
    pub fn get(uploaded_at: DateTime<Utc>, data: Vec<u8>) -> Self {
        FetchOk {
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
    async fn set_url(&mut self, url: &str) -> FetchResult;
}
