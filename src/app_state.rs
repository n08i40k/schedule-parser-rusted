use crate::xls_downloader::basic_impl::BasicXlsDownloader;
use actix_web::web;
use chrono::{DateTime, Utc};
use diesel::{Connection, PgConnection};
use std::env;
use std::sync::{Mutex, MutexGuard};
use crate::parser::schema::ParseResult;

pub struct Schedule {
    pub etag: String,
    pub updated_at: DateTime<Utc>,
    pub parsed_at: DateTime<Utc>,
    pub data: ParseResult,
}

pub struct AppState {
    pub downloader: Mutex<BasicXlsDownloader>,
    pub schedule: Mutex<Option<Schedule>>,
    pub database: Mutex<PgConnection>,
}

impl AppState {
    pub fn connection(&self) -> MutexGuard<PgConnection> {
        self.database.lock().unwrap()
    }
}

pub fn app_state() -> web::Data<AppState> {
    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    web::Data::new(AppState {
        downloader: Mutex::new(BasicXlsDownloader::new()),
        schedule: Mutex::new(None),
        database: Mutex::new(
            PgConnection::establish(&database_url)
                .unwrap_or_else(|_| panic!("Error connecting to {}", database_url)),
        ),
    })
}
