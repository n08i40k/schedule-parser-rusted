use crate::parser::parse_xls;
use crate::updater::error::{Error, QueryUrlError, SnapshotCreationError};
use crate::xls_downloader::{FetchError, XlsDownloader};
use base::ScheduleSnapshot;

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

pub mod error {
    use crate::xls_downloader::FetchError;
    use derive_more::{Display, Error};

    #[derive(Debug, Display, Error)]
    pub enum Error {
        /// An error occurred while querying the Yandex Cloud API for a URL.
        ///
        /// This may result from network failures, invalid API credentials, or issues with the Yandex Cloud Function invocation.
        /// See [`QueryUrlError`] for more details about specific causes.
        QueryUrlFailed(QueryUrlError),

        /// The schedule snapshot creation process failed.
        ///
        /// This can happen due to URL conflicts (same URL already in use), failed network requests,
        /// download errors, or invalid XLS file content. See [`SnapshotCreationError`] for details.
        SnapshotCreationFailed(SnapshotCreationError),
    }
    /// Errors that may occur when querying the Yandex Cloud API to retrieve a URL.
    #[derive(Debug, Display, Error)]
    pub enum QueryUrlError {
        /// Occurs when the request to the Yandex Cloud API fails.
        ///
        /// This may be due to network issues, invalid API key, incorrect function ID, or other
        /// problems with the Yandex Cloud Function invocation.
        #[display("An error occurred during the request to the Yandex Cloud API: {_0}")]
        RequestFailed(reqwest::Error),
    }

    /// Errors that may occur during the creation of a schedule snapshot.
    #[derive(Debug, Display, Error)]
    pub enum SnapshotCreationError {
        /// The URL is the same as the one already being used (no update needed).
        #[display("The URL is the same as the one already being used.")]
        SameUrl,

        /// The URL query for the XLS file failed to execute, either due to network issues or invalid API parameters.
        #[display("Failed to fetch URL: {_0}")]
        FetchFailed(FetchError),

        /// Downloading the XLS file content failed after successfully obtaining the URL.
        #[display("Download failed: {_0}")]
        DownloadFailed(FetchError),

        /// The XLS file could not be parsed into a valid schedule format.
        #[display("Schedule data is invalid: {_0}")]
        InvalidSchedule(crate::parser::error::Error),
    }
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
    pub async fn new_snapshot(
        downloader: &mut XlsDownloader,
        url: String,
    ) -> Result<ScheduleSnapshot, SnapshotCreationError> {
        if downloader.url.as_ref().is_some_and(|_url| _url.eq(&url)) {
            return Err(SnapshotCreationError::SameUrl);
        }

        let head_result = downloader.set_url(&*url).await.map_err(|error| {
            if let FetchError::Unknown(error) = &error {
                sentry::capture_error(&error);
            }

            SnapshotCreationError::FetchFailed(error)
        })?;

        let xls_data = downloader
            .fetch(false)
            .await
            .map_err(|error| {
                if let FetchError::Unknown(error) = &error {
                    sentry::capture_error(&error);
                }

                SnapshotCreationError::DownloadFailed(error)
            })?
            .data
            .unwrap();

        let parse_result = parse_xls(&xls_data).map_err(|error| {
            sentry::capture_error(&error);

            SnapshotCreationError::InvalidSchedule(error)
        })?;

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
    async fn query_url(api_key: &str, func_id: &str) -> Result<String, QueryUrlError> {
        let client = reqwest::Client::new();

        let uri = client
            .post(format!(
                "https://functions.yandexcloud.net/{}?integration=raw",
                func_id
            ))
            .header("Authorization", format!("Api-Key {}", api_key))
            .send()
            .await
            .map_err(|error| QueryUrlError::RequestFailed(error))?
            .text()
            .await
            .map_err(|error| QueryUrlError::RequestFailed(error))?;

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
    pub async fn new(update_source: UpdateSource) -> Result<(Self, ScheduleSnapshot), Error> {
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
                Self::query_url(yandex_api_key, yandex_func_id)
                    .await
                    .map_err(|error| Error::QueryUrlFailed(error))?
            }
            _ => unreachable!(),
        };

        log::info!("For the initial setup, a link {} will be used", url);

        let snapshot = Self::new_snapshot(&mut this.downloader, url)
            .await
            .map_err(|error| Error::SnapshotCreationFailed(error))?;

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
    ) -> Result<ScheduleSnapshot, Error> {
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
            } => Self::query_url(yandex_api_key.as_str(), yandex_func_id.as_str())
                .await
                .map_err(|error| Error::QueryUrlFailed(error))?,
            _ => unreachable!(),
        };

        let snapshot = match Self::new_snapshot(&mut self.downloader, url).await {
            Ok(snapshot) => snapshot,
            Err(SnapshotCreationError::SameUrl) => {
                let mut clone = current_snapshot.clone();
                clone.update();

                clone
            }
            Err(error) => return Err(Error::SnapshotCreationFailed(error)),
        };

        Ok(snapshot)
    }
}
