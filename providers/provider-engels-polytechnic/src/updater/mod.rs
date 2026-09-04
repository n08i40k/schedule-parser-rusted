pub use self::error::{Error, Result};
use crate::parser::parse_xls;
use crate::xls_downloader::{FetchError, Source};
use base::ScheduleSnapshot;
use chrono::Utc;
mod error;

pub enum UpdateSource {
    Prepared(ScheduleSnapshot),

    /// Public Yandex Disk folder the college uploads the schedule to.
    YandexDisk {
        public_url: String,
    },
}

pub struct Updater {
    update_source: UpdateSource,

    /// Version of the file the current snapshot was built from.
    version: Option<String>,
}

impl Updater {
    /// Place the schedule is downloaded from, or [`None`] for a prepared snapshot.
    fn source(&self) -> Option<Source> {
        match &self.update_source {
            UpdateSource::Prepared(_) => None,
            UpdateSource::YandexDisk { public_url } => Some(Source::new(public_url.clone())),
        }
    }

    /// Constructs a new [`ScheduleSnapshot`] by downloading and parsing the current schedule file.
    ///
    /// The file is looked up first, and its content is downloaded only when the version marker
    /// differs from the one the current snapshot was built from.
    ///
    /// # Returns
    ///
    /// Returns [`Error::NotModified`] when the remote file has not changed since the last update,
    /// or an error describing the failed download or parsing.
    async fn new_snapshot(&mut self) -> Result<ScheduleSnapshot> {
        let source = self.source().expect("a prepared snapshot has no source");

        let file = source.probe().await.map_err(|error| {
            if let FetchError::Reqwest(error) = &error {
                sentry::capture_error(&error);
            }

            Error::ScheduleFetchFailed(error)
        })?;

        if self.version.as_deref() == Some(file.version.as_str()) {
            return Err(Error::NotModified);
        }

        let xls_data = source.download(&file).await.map_err(|error| {
            if let FetchError::Reqwest(error) = &error {
                sentry::capture_error(&error);
            }

            Error::ScheduleDownloadFailed(error)
        })?;

        let parse_result = parse_xls(&xls_data)?;

        self.version = Some(file.version);

        Ok(ScheduleSnapshot {
            fetched_at: Utc::now(),
            updated_at: file.modified_at,
            url: file.url,
            data: parse_result,
        })
    }

    /// Initializes the schedule by downloading the current file from the configured source.
    ///
    /// # Arguments
    ///
    /// * `update_source`: Place the schedule is taken from.
    ///
    /// # Returns
    ///
    /// Returns the updater together with the initial [`ScheduleSnapshot`], or an error if the
    /// schedule could not be downloaded or parsed.
    pub async fn new(update_source: UpdateSource) -> Result<(Self, ScheduleSnapshot)> {
        let mut this = Updater {
            update_source,
            version: None,
        };

        if let UpdateSource::Prepared(snapshot) = &this.update_source {
            let snapshot = snapshot.clone();
            return Ok((this, snapshot));
        }

        log::info!("Creating the initial schedule snapshot...");

        let snapshot = this.new_snapshot().await?;
        log::info!("Schedule snapshot successfully created!");

        Ok((this, snapshot))
    }

    /// Rebuilds the schedule snapshot from the current remote file.
    ///
    /// When the remote file has not changed, the current snapshot is reused with a refreshed
    /// fetch timestamp.
    ///
    /// # Arguments
    ///
    /// * `current_snapshot`: Snapshot the provider currently serves.
    ///
    /// returns: `Result<ScheduleSnapshot, Error>`
    pub async fn update(
        &mut self,
        current_snapshot: &ScheduleSnapshot,
    ) -> Result<ScheduleSnapshot> {
        if let UpdateSource::Prepared(snapshot) = &self.update_source {
            let mut snapshot = snapshot.clone();
            snapshot.update();
            return Ok(snapshot);
        }

        let snapshot = match self.new_snapshot().await {
            Ok(snapshot) => snapshot,
            Err(Error::NotModified) => {
                let mut clone = current_snapshot.clone();
                clone.update();

                clone
            }
            Err(error) => return Err(error),
        };

        Ok(snapshot)
    }
}
