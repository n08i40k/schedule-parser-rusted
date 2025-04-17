use crate::xls_downloader::interface::{FetchError, FetchOk, FetchResult, XLSDownloader};
use chrono::{DateTime, Utc};
use std::env;

pub struct BasicXlsDownloader {
    pub url: Option<String>,
    user_agent: String,
}

async fn fetch_specified(url: &String, user_agent: &String, head: bool) -> FetchResult {
    let client = reqwest::Client::new();

    let response = if head {
        client.head(url)
    } else {
        client.get(url)
    }
    .header("User-Agent", user_agent.clone())
    .send()
    .await;

    match response {
        Ok(r) => {
            if r.status().as_u16() != 200 {
                return Err(FetchError::BadStatusCode);
            }

            let headers = r.headers();

            let content_type = headers.get("Content-Type");
            let etag = headers.get("etag");
            let last_modified = headers.get("last-modified");
            let date = headers.get("date");

            if content_type.is_none() || etag.is_none() || last_modified.is_none() || date.is_none()
            {
                Err(FetchError::BadHeaders)
            } else if content_type.unwrap() != "application/vnd.ms-excel" {
                Err(FetchError::BadContentType)
            } else {
                let etag = etag.unwrap().to_str().unwrap().to_string();
                let last_modified =
                    DateTime::parse_from_rfc2822(&last_modified.unwrap().to_str().unwrap())
                        .unwrap()
                        .with_timezone(&Utc);

                Ok(if head {
                    FetchOk::head(etag, last_modified)
                } else {
                    FetchOk::get(etag, last_modified, r.bytes().await.unwrap().to_vec())
                })
            }
        }
        Err(e) => Err(FetchError::Unknown(e)),
    }
}

impl BasicXlsDownloader {
    pub fn new() -> Self {
        BasicXlsDownloader {
            url: None,
            user_agent: env::var("REQWEST_USER_AGENT").expect("USER_AGENT must be set"),
        }
    }
}

impl XLSDownloader for BasicXlsDownloader {
    async fn fetch(&self, head: bool) -> FetchResult {
        if self.url.is_none() {
            Err(FetchError::NoUrlProvided)
        } else {
            fetch_specified(self.url.as_ref().unwrap(), &self.user_agent, head).await
        }
    }

    async fn set_url(&mut self, url: String) -> FetchResult {
        let result = fetch_specified(&url, &self.user_agent, true).await;

        if let Ok(_) = result {
            self.url = Some(url);
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
        let url = "bad_url".to_string();
        let user_agent = String::new();

        let results = [
            fetch_specified(&url, &user_agent, true).await,
            fetch_specified(&url, &user_agent, false).await,
        ];

        assert!(results[0].is_err());
        assert!(results[1].is_err());
    }

    #[tokio::test]
    async fn bad_status_code() {
        let url = "https://www.google.com/not-found".to_string();
        let user_agent = String::new();

        let results = [
            fetch_specified(&url, &user_agent, true).await,
            fetch_specified(&url, &user_agent, false).await,
        ];

        assert!(results[0].is_err());
        assert!(results[1].is_err());

        assert_eq!(
            *results[0].as_ref().err().unwrap(),
            FetchError::BadStatusCode
        );
        assert_eq!(
            *results[1].as_ref().err().unwrap(),
            FetchError::BadStatusCode
        );
    }

    #[tokio::test]
    async fn bad_headers() {
        let url = "https://www.google.com/favicon.ico".to_string();
        let user_agent = String::new();

        let results = [
            fetch_specified(&url, &user_agent, true).await,
            fetch_specified(&url, &user_agent, false).await,
        ];

        assert!(results[0].is_err());
        assert!(results[1].is_err());

        assert_eq!(*results[0].as_ref().err().unwrap(), FetchError::BadHeaders);
        assert_eq!(*results[1].as_ref().err().unwrap(), FetchError::BadHeaders);
    }

    #[tokio::test]
    async fn bad_content_type() {
        let url = "https://s3.aero-storage.ldragol.ru/679e5d1145a6ad00843ad3f1/67ddb59fd46303008396ac96%2Fexample.txt".to_string();
        let user_agent = String::new();

        let results = [
            fetch_specified(&url, &user_agent, true).await,
            fetch_specified(&url, &user_agent, false).await,
        ];

        assert!(results[0].is_err());
        assert!(results[1].is_err());

        assert_eq!(
            *results[0].as_ref().err().unwrap(),
            FetchError::BadContentType
        );
        assert_eq!(
            *results[1].as_ref().err().unwrap(),
            FetchError::BadContentType
        );
    }

    #[tokio::test]
    async fn ok() {
        let url = "https://s3.aero-storage.ldragol.ru/679e5d1145a6ad00843ad3f1/67ddb5fad46303008396ac97%2Fschedule.xls".to_string();
        let user_agent = String::new();

        let results = [
            fetch_specified(&url, &user_agent, true).await,
            fetch_specified(&url, &user_agent, false).await,
        ];

        assert!(results[0].is_ok());
        assert!(results[1].is_ok());
    }

    #[tokio::test]
    async fn downloader_set_ok() {
        let url = "https://s3.aero-storage.ldragol.ru/679e5d1145a6ad00843ad3f1/67ddb5fad46303008396ac97%2Fschedule.xls".to_string();

        let mut downloader = BasicXlsDownloader::new();

        assert!(downloader.set_url(url).await.is_ok());
    }

    #[tokio::test]
    async fn downloader_set_err() {
        let url = "bad_url".to_string();

        let mut downloader = BasicXlsDownloader::new();

        assert!(downloader.set_url(url).await.is_err());
    }

    #[tokio::test]
    async fn downloader_ok() {
        let url = "https://s3.aero-storage.ldragol.ru/679e5d1145a6ad00843ad3f1/67ddb5fad46303008396ac97%2Fschedule.xls".to_string();

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
