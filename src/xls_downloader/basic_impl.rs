use crate::xls_downloader::interface::{FetchError, FetchOk, FetchResult, XLSDownloader};
use chrono::{DateTime, Utc};
use std::sync::Arc;

pub struct BasicXlsDownloader {
    pub url: Option<String>,
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

    let last_modified = DateTime::parse_from_rfc2822(&last_modified.to_str().unwrap())
        .unwrap()
        .with_timezone(&Utc);

    Ok(if head {
        FetchOk::head(last_modified)
    } else {
        FetchOk::get(last_modified, response.bytes().await.unwrap().to_vec())
    })
}

impl BasicXlsDownloader {
    pub fn new() -> Self {
        BasicXlsDownloader { url: None }
    }
}

impl XLSDownloader for BasicXlsDownloader {
    async fn fetch(&self, head: bool) -> FetchResult {
        if self.url.is_none() {
            Err(FetchError::NoUrlProvided)
        } else {
            fetch_specified(&*self.url.as_ref().unwrap(), head).await
        }
    }

    async fn set_url(&mut self, url: &str) -> FetchResult {
        let result = fetch_specified(url, true).await;

        if let Ok(_) = result {
            self.url = Some(url.to_string());
        }

        result
    }
}

#[cfg(test)]
mod tests {
    use crate::xls_downloader::basic_impl::{BasicXlsDownloader, fetch_specified};
    use crate::xls_downloader::interface::{FetchError, XLSDownloader};

    #[tokio::test]
    async fn bad_url() {
        let url = "bad_url";

        let results = [
            fetch_specified(url, true).await,
            fetch_specified(url, false).await,
        ];

        assert!(results[0].is_err());
        assert!(results[1].is_err());
    }

    #[tokio::test]
    async fn bad_status_code() {
        let url = "https://www.google.com/not-found";

        let results = [
            fetch_specified(url, true).await,
            fetch_specified(url, false).await,
        ];

        assert!(results[0].is_err());
        assert!(results[1].is_err());

        let expected_error = FetchError::BadStatusCode { status_code: 404 };

        assert_eq!(*results[0].as_ref().err().unwrap(), expected_error);
        assert_eq!(*results[1].as_ref().err().unwrap(), expected_error);
    }

    #[tokio::test]
    async fn bad_headers() {
        let url = "https://www.google.com/favicon.ico";

        let results = [
            fetch_specified(url, true).await,
            fetch_specified(url, false).await,
        ];

        assert!(results[0].is_err());
        assert!(results[1].is_err());

        let expected_error = FetchError::BadHeaders {
            expected_header: "ETag".to_string(),
        };

        assert_eq!(*results[0].as_ref().err().unwrap(), expected_error);
        assert_eq!(*results[1].as_ref().err().unwrap(), expected_error);
    }

    #[tokio::test]
    async fn bad_content_type() {
        let url = "https://s3.aero-storage.ldragol.ru/679e5d1145a6ad00843ad3f1/67ddb59fd46303008396ac96%2Fexample.txt";

        let results = [
            fetch_specified(url, true).await,
            fetch_specified(url, false).await,
        ];

        assert!(results[0].is_err());
        assert!(results[1].is_err());
    }

    #[tokio::test]
    async fn ok() {
        let url = "https://s3.aero-storage.ldragol.ru/679e5d1145a6ad00843ad3f1/67ddb5fad46303008396ac97%2Fschedule.xls";

        let results = [
            fetch_specified(url, true).await,
            fetch_specified(url, false).await,
        ];

        assert!(results[0].is_ok());
        assert!(results[1].is_ok());
    }

    #[tokio::test]
    async fn downloader_set_ok() {
        let url = "https://s3.aero-storage.ldragol.ru/679e5d1145a6ad00843ad3f1/67ddb5fad46303008396ac97%2Fschedule.xls";

        let mut downloader = BasicXlsDownloader::new();

        assert!(downloader.set_url(url).await.is_ok());
    }

    #[tokio::test]
    async fn downloader_set_err() {
        let url = "bad_url";

        let mut downloader = BasicXlsDownloader::new();

        assert!(downloader.set_url(url).await.is_err());
    }

    #[tokio::test]
    async fn downloader_ok() {
        let url = "https://s3.aero-storage.ldragol.ru/679e5d1145a6ad00843ad3f1/67ddb5fad46303008396ac97%2Fschedule.xls";

        let mut downloader = BasicXlsDownloader::new();

        assert!(downloader.set_url(url).await.is_ok());
        assert!(downloader.fetch(false).await.is_ok());
    }

    #[tokio::test]
    async fn downloader_no_url_provided() {
        let downloader = BasicXlsDownloader::new();
        let result = downloader.fetch(false).await;

        assert!(result.is_err());
        assert_eq!(result.err().unwrap(), FetchError::NoUrlProvided);
    }
}
