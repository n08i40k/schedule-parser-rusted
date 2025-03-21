use chrono::{DateTime, Utc};

#[derive(PartialEq, Debug)]
pub enum FetchError {
    NoUrlProvided,
    Unknown,
    BadStatusCode,
    BadContentType,
    BadHeaders,
}

pub struct FetchOk {
    pub etag: String,
    pub uploaded_at: DateTime<Utc>,
    pub requested_at: DateTime<Utc>,
    pub data: Option<Vec<u8>>,
}

impl FetchOk {
    pub fn head(etag: String, uploaded_at: DateTime<Utc>) -> Self {
        FetchOk {
            etag,
            uploaded_at,
            requested_at: Utc::now(),
            data: None,
        }
    }

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
    async fn fetch(&self, head: bool) -> FetchResult;
    async fn set_url(&mut self, url: String) -> Result<(), FetchError>;
}
