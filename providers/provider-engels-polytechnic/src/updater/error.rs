use crate::xls_downloader::FetchError;
use derive_more::{Display, Error, From};

#[derive(Debug, Display, Error, From)]
pub enum Error {
    /// Occurs when the request to the Yandex Cloud API fails.
    ///
    /// This may be due to network issues, invalid API key, incorrect function ID, or other
    /// problems with the Yandex Cloud Function invocation.
    #[display("An error occurred during the request to the Yandex Cloud API: {_0}")]
    Reqwest(reqwest::Error),

    #[display("Unable to get URI in 3 retries")]
    EmptyUri,

    /// The ETag is the same (no update needed).
    #[display("The ETag is the same.")]
    SameETag,

    /// The URL query for the XLS file failed to execute, either due to network issues or invalid API parameters.
    #[display("Failed to fetch URL: {_0}")]
    ScheduleFetchFailed(FetchError),

    /// Downloading the XLS file content failed after successfully obtaining the URL.
    #[display("Download failed: {_0}")]
    ScheduleDownloadFailed(FetchError),

    /// The XLS file could not be parsed into a valid schedule format.
    #[from]
    InvalidSchedule(crate::parser::Error),
}

pub type Result<T> = core::result::Result<T, Error>;
