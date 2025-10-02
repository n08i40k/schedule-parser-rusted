pub use self::error::{Error, Result};
use crate::parser::parse_xls;
use crate::xls_downloader::{FetchError, XlsDownloader};
use base::ScheduleSnapshot;
mod error;

pub enum UpdateSource {
    Prepared(ScheduleSnapshot),

    Url(String),

    GrabFromSite {
        yandex_api_key: String,
        yandex_func_id: String,
    },
}

pub struct Updater {
    downloader: XlsDownloader,
    update_source: UpdateSource,
}

impl Updater {
    /// Constructs a new `ScheduleSnapshot` by downloading and parsing schedule data from the specified URL.
    ///
    /// This method first checks if the provided URL is the same as the one already configured in the downloader.
    /// If different, it updates the downloader's URL, fetches the XLS content, parses it, and creates a snapshot.
    /// Errors are returned for URL conflicts, network issues, download failures, or invalid data.
    ///
    /// # Arguments
    ///
    /// * `downloader`: A mutable reference to an `XLSDownloader` implementation used to fetch and parse the schedule data.
    /// * `url`: The source URL pointing to the XLS file containing schedule data.
    ///
    /// returns: Result<ScheduleSnapshot, SnapshotCreationError>
    async fn new_snapshot(downloader: &mut XlsDownloader, url: String) -> Result<ScheduleSnapshot> {
        let head_result = downloader.set_url(&url).await.map_err(|error| {
            if let FetchError::Reqwest(error) = &error {
                sentry::capture_error(&error);
            }

            Error::ScheduleFetchFailed(error)
        })?;

        if downloader.etag == Some(head_result.etag) {
            return Err(Error::SameETag);
        }

        let xls_data = downloader
            .fetch(false)
            .await
            .map_err(|error| {
                if let FetchError::Reqwest(error) = &error {
                    sentry::capture_error(&error);
                }

                Error::ScheduleDownloadFailed(error)
            })?
            .data
            .unwrap();

        let parse_result = parse_xls(&xls_data)?;

        Ok(ScheduleSnapshot {
            fetched_at: head_result.requested_at,
            updated_at: head_result.uploaded_at,
            url,
            data: parse_result,
        })
    }

    /// Queries the Yandex Cloud Function (FaaS) to obtain a URL for the schedule file.
    ///
    /// This sends a POST request to the specified Yandex Cloud Function endpoint,
    /// using the provided API key for authentication. The returned URI is combined
    /// with the "https://politehnikum-eng.ru" base domain to form the complete URL.
    ///
    /// # Arguments
    ///
    /// * `api_key` - Authentication token for Yandex Cloud API
    /// * `func_id` - ID of the target Yandex Cloud Function to invoke
    ///
    /// # Returns
    ///
    /// Result containing:
    /// - `Ok(String)` - Complete URL constructed from the Function's response
    /// - `Err(QueryUrlError)` - If the request or response processing fails
    async fn query_url(api_key: &str, func_id: &str) -> Result<String> {
        let client = reqwest::Client::new();

        let uri = {
            // вот бы добавили named-scopes как в котлине,
            // чтоб мне не пришлось такой хуйнёй страдать.
            #[allow(unused_assignments)]
            let mut uri = String::new();
            let mut counter = 0;

            loop {
                if counter == 3 {
                    return Err(Error::EmptyUri);
                }

                counter += 1;

                uri = client
                    .post(format!(
                        "https://functions.yandexcloud.net/{}?integration=raw",
                        func_id
                    ))
                    .header("Authorization", format!("Api-Key {}", api_key))
                    .send()
                    .await
                    .map_err(Error::Reqwest)?
                    .text()
                    .await
                    .map_err(Error::Reqwest)?;

                if uri.is_empty() {
                    log::warn!("[{}] Unable to get uri! Retrying in 5 seconds...", counter);
                    continue;
                }

                break;
            }

            uri
        };

        Ok(format!("https://politehnikum-eng.ru{}", uri.trim()))
    }

    /// Initializes the schedule by fetching the URL from the environment or Yandex Cloud Function (FaaS)
    /// and creating a [`ScheduleSnapshot`] with the downloaded data.
    ///
    /// # Arguments
    ///
    /// * `downloader`: Mutable reference to an `XLSDownloader` implementation used to fetch and parse the schedule
    /// * `app_env`: Reference to the application environment containing either a predefined URL or Yandex Cloud credentials
    ///
    /// # Returns
    ///
    /// Returns `Ok(())` if the snapshot was successfully initialized, or an `Error` if:
    /// - URL query to Yandex Cloud failed ([`QueryUrlError`])
    /// - Schedule snapshot creation failed ([`SnapshotCreationError`])
    pub async fn new(update_source: UpdateSource) -> Result<(Self, ScheduleSnapshot)> {
        let mut this = Updater {
            downloader: XlsDownloader::new(),
            update_source,
        };

        if let UpdateSource::Prepared(snapshot) = &this.update_source {
            let snapshot = snapshot.clone();
            return Ok((this, snapshot));
        }

        let url = match &this.update_source {
            UpdateSource::Url(url) => {
                log::info!("The default link {} will be used", url);
                url.clone()
            }
            UpdateSource::GrabFromSite {
                yandex_api_key,
                yandex_func_id,
            } => {
                log::info!("Obtaining a link using FaaS...");
                Self::query_url(yandex_api_key, yandex_func_id).await?
            }
            _ => unreachable!(),
        };

        log::info!("For the initial setup, a link {} will be used", url);

        let snapshot = Self::new_snapshot(&mut this.downloader, url).await?;
        log::info!("Schedule snapshot successfully created!");

        Ok((this, snapshot))
    }

    /// Updates the schedule snapshot by querying the latest URL from FaaS and checking for changes.
    /// If the URL hasn't changed, only updates the [`fetched_at`] timestamp. If changed, downloads
    /// and parses the new schedule data.
    ///
    /// # Arguments
    ///
    /// * `downloader`: XLS file downloader used to fetch and parse the schedule data
    /// * `app_env`: Application environment containing Yandex Cloud configuration and auto-update settings
    ///
    /// returns: `Result<(), Error>` - Returns error if URL query fails or schedule parsing encounters issues
    ///
    /// # Safety
    ///
    /// Use `unsafe` to access the initialized snapshot, guaranteed valid by prior `init()` call
    pub async fn update(
        &mut self,
        current_snapshot: &ScheduleSnapshot,
    ) -> Result<ScheduleSnapshot> {
        if let UpdateSource::Prepared(snapshot) = &self.update_source {
            let mut snapshot = snapshot.clone();
            snapshot.update();
            return Ok(snapshot);
        }

        let url = match &self.update_source {
            UpdateSource::Url(url) => url.clone(),
            UpdateSource::GrabFromSite {
                yandex_api_key,
                yandex_func_id,
            } => Self::query_url(yandex_api_key.as_str(), yandex_func_id.as_str()).await?,
            _ => unreachable!(),
        };

        let snapshot = match Self::new_snapshot(&mut self.downloader, url).await {
            Ok(snapshot) => snapshot,
            Err(Error::SameETag) => {
                let mut clone = current_snapshot.clone();
                clone.update();

                clone
            }
            Err(error) => return Err(error),
        };

        Ok(snapshot)
    }
}
