use crate::updater::Updater;
use async_trait::async_trait;
use base::{ScheduleProvider, ScheduleSnapshot};
use std::ops::DerefMut;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tokio::time::interval;
use tokio_util::sync::CancellationToken;

pub use crate::updater::UpdateSource;

mod parser;
mod updater;
mod xls_downloader;

#[cfg(feature = "test")]
pub mod test_utils {
    pub use crate::parser::test_utils::test_result;
}

pub struct EngelsPolytechnicProvider {
    updater: Updater,
    snapshot: Arc<ScheduleSnapshot>,
}

impl EngelsPolytechnicProvider {
    pub async fn get(
        update_source: UpdateSource,
    ) -> Result<Arc<dyn ScheduleProvider>, crate::updater::error::Error> {
        let (updater, snapshot) = Updater::new(update_source).await?;

        Ok(Arc::new(Wrapper {
            inner: RwLock::new(Self {
                updater,
                snapshot: Arc::new(snapshot),
            }),
        }))
    }
}

pub struct Wrapper {
    inner: RwLock<EngelsPolytechnicProvider>,
}

#[async_trait]
impl ScheduleProvider for Wrapper {
    async fn start_auto_update_task(
        &self,
        cancellation_token: CancellationToken,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>> {
        let mut ticker = interval(Duration::from_secs(60 * 30));
        ticker.tick().await; // bc we already have the latest schedule, when instantiating provider

        loop {
            tokio::select! {
                _ = ticker.tick() => {
                    let mut lock = self.inner.write().await;
                    let this= lock.deref_mut();

                    log::info!("Updating schedule...");

                    match this.updater.update(&this.snapshot).await {
                        Ok(snapshot) => {
                            this.snapshot = Arc::new(snapshot);
                        },

                        Err(updater::error::Error::QueryUrlFailed(updater::error::QueryUrlError::UriFetchFailed)) => {},

                        Err(err) => {
                            sentry::capture_error(&err);
                        }
                    }
                }

                _ = cancellation_token.cancelled() => {
                    return Ok(());
                }
            }
        }
    }

    async fn get_schedule(&self) -> Arc<ScheduleSnapshot> {
        self.inner.read().await.snapshot.clone()
    }
}
