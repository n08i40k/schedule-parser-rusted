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
#[derive(Debug, PartialEq)]
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

pub struct XlsDownloader {
    pub url: Option<String>,
}

impl XlsDownloader {
    pub fn new() -> Self {
        XlsDownloader { url: None }
    }

    async fn fetch_specified(url: &str, head: bool) -> FetchResult {
        let client = reqwest::Client::new();

        let response = if head {
            client.head(url)
        } else {
            client.get(url)
        }
        .header("User-Agent", ua_generator::ua::spoof_chrome_ua())
        .send()
        .await
        .map_err(|e| FetchError::unknown(Arc::new(e)))?;

        if response.status().as_u16() != 200 {
            return Err(FetchError::bad_status_code(response.status().as_u16()));
        }

        let headers = response.headers();

        let content_type = headers
            .get("Content-Type")
            .ok_or(FetchError::bad_headers("Content-Type"))?;

        if !headers.contains_key("etag") {
            return Err(FetchError::bad_headers("etag"));
        }

        let last_modified = headers
            .get("last-modified")
            .ok_or(FetchError::bad_headers("last-modified"))?;

        if content_type != "application/vnd.ms-excel" {
            return Err(FetchError::bad_content_type(content_type.to_str().unwrap()));
        }

        let last_modified = DateTime::parse_from_rfc2822(last_modified.to_str().unwrap())
            .unwrap()
            .with_timezone(&Utc);

        Ok(if head {
            FetchOk::head(last_modified)
        } else {
            FetchOk::get(last_modified, response.bytes().await.unwrap().to_vec())
        })
    }

    pub async fn fetch(&self, head: bool) -> FetchResult {
        if self.url.is_none() {
            Err(FetchError::NoUrlProvided)
        } else {
            Self::fetch_specified(self.url.as_ref().unwrap(), head).await
        }
    }

    pub async fn set_url(&mut self, url: &str) -> FetchResult {
        let result = Self::fetch_specified(url, true).await;

        if result.is_ok() {
            self.url = Some(url.to_string());
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use crate::xls_downloader::{FetchError, XlsDownloader};

    #[tokio::test]
    async fn bad_url() {
        let url = "bad_url";

        let mut downloader = XlsDownloader::new();
        assert!(downloader.set_url(url).await.is_err());
    }

    #[tokio::test]
    async fn bad_status_code() {
        let url = "https://www.google.com/not-found";

        let mut downloader = XlsDownloader::new();
        assert_eq!(
            downloader.set_url(url).await,
            Err(FetchError::bad_status_code(404))
        );
    }

    #[tokio::test]
    async fn bad_headers() {
        let url = "https://www.google.com/favicon.ico";

        let mut downloader = XlsDownloader::new();
        assert_eq!(
            downloader.set_url(url).await,
            Err(FetchError::BadHeaders {
                expected_header: "ETag".to_string(),
            })
        );
    }

    #[tokio::test]
    async fn bad_content_type() {
        let url = "https://s3.aero-storage.ldragol.ru/679e5d1145a6ad00843ad3f1/67ddb59fd46303008396ac96%2Fexample.txt";

        let mut downloader = XlsDownloader::new();
        assert!(downloader.set_url(url).await.is_err());
    }

    #[tokio::test]
    async fn ok() {
        let url = "https://s3.aero-storage.ldragol.ru/679e5d1145a6ad00843ad3f1/67ddb5fad46303008396ac97%2Fschedule.xls";

        let mut downloader = XlsDownloader::new();
        assert!(downloader.set_url(url).await.is_ok());
    }

    #[tokio::test]
    async fn downloader_ok() {
        let url = "https://s3.aero-storage.ldragol.ru/679e5d1145a6ad00843ad3f1/67ddb5fad46303008396ac97%2Fschedule.xls";

        let mut downloader = XlsDownloader::new();
        assert!(downloader.set_url(url).await.is_ok());
        assert!(downloader.fetch(false).await.is_ok());
    }

    #[tokio::test]
    async fn downloader_no_url_provided() {
        let downloader = XlsDownloader::new();

        let result = downloader.fetch(false).await;
        assert_eq!(result, Err(FetchError::NoUrlProvided));
    }
}
