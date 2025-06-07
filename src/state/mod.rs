mod env;
mod fcm_client;
mod schedule;

use crate::state::fcm_client::FCMClientData;
use crate::xls_downloader::basic_impl::BasicXlsDownloader;
use actix_web::web;
use diesel::{Connection, PgConnection};
use firebase_messaging_rs::FCMClient;
use std::ops::DerefMut;
use tokio::sync::{MappedMutexGuard, Mutex, MutexGuard};

pub use self::schedule::{Schedule, ScheduleSnapshot};
pub use crate::state::env::AppEnv;

/// Common data provided to endpoints.
pub struct AppState {
    database: Mutex<PgConnection>,
    downloader: Mutex<BasicXlsDownloader>,
    schedule: Mutex<Schedule>,
    env: AppEnv,
    fcm_client: Option<Mutex<FCMClient>>,
}

impl AppState {
    pub async fn new() -> Result<Self, self::schedule::Error> {
        let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

        let mut _self = Self {
            downloader: Mutex::new(BasicXlsDownloader::new()),

            schedule: Mutex::new(Schedule::default()),
            database: Mutex::new(
                PgConnection::establish(&database_url)
                    .unwrap_or_else(|_| panic!("Error connecting to {}", database_url)),
            ),
            env: AppEnv::default(),
            fcm_client: FCMClientData::new().await,
        };

        if _self.env.schedule.auto_update {
            _self
                .get_schedule()
                .await
                .init(_self.get_downloader().await.deref_mut(), &_self.env)
                .await?;
        }

        Ok(_self)
    }

    pub async fn get_downloader(&'_ self) -> MutexGuard<'_, BasicXlsDownloader> {
        self.downloader.lock().await
    }

    pub async fn get_schedule(&'_ self) -> MutexGuard<'_, Schedule> {
        self.schedule.lock().await
    }

    pub async fn get_schedule_snapshot(&'_ self) -> MappedMutexGuard<'_, ScheduleSnapshot> {
        let snapshot =
            MutexGuard::<'_, Schedule>::map(self.schedule.lock().await, |schedule| unsafe {
                schedule.snapshot.assume_init_mut()
            });

        snapshot
    }

    pub async fn get_database(&'_ self) -> MutexGuard<'_, PgConnection> {
        self.database.lock().await
    }

    pub fn get_env(&self) -> &AppEnv {
        &self.env
    }

    pub async fn get_fcm_client(&'_ self) -> Option<MutexGuard<'_, FCMClient>> {
        match &self.fcm_client {
            Some(client) => Some(client.lock().await),
            None => None,
        }
    }
}

/// Create a new object web::Data<AppState>.
pub async fn new_app_state() -> Result<web::Data<AppState>, self::schedule::Error> {
    Ok(web::Data::new(AppState::new().await?))
}
