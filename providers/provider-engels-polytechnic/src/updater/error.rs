use crate::xls_downloader::FetchError;
use derive_more::{Display, Error, From};

#[derive(Debug, Display, Error, From)]
pub enum Error {
    /// The remote file has not changed since the last update.
    #[display("The schedule file has not changed.")]
    NotModified,

    /// The lookup of the current schedule file failed, either due to network issues or an
    /// unexpected response from the storage.
    #[display("Failed to fetch URL: {_0}")]
    ScheduleFetchFailed(FetchError),

    /// Downloading the XLS file content failed after successfully locating the file.
    #[display("Download failed: {_0}")]
    ScheduleDownloadFailed(FetchError),

    /// The XLS file could not be parsed into a valid schedule format.
    #[from]
    InvalidSchedule(crate::parser::Error),
}

pub type Result<T> = core::result::Result<T, Error>;
