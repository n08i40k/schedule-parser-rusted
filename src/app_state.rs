use crate::parser::schema::ParseResult;
use crate::utility::hasher::DigestHasher;
use crate::xls_downloader::basic_impl::BasicXlsDownloader;
use actix_web::web;
use chrono::{DateTime, Utc};
use diesel::{Connection, PgConnection};
use sha1::{Digest, Sha1};
use std::env;
use std::hash::Hash;
use std::sync::{Mutex, MutexGuard};

#[derive(Clone)]
pub struct Schedule {
    pub etag: String,
    pub fetched_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub parsed_at: DateTime<Utc>,
    pub data: ParseResult,
}

impl Schedule {
    pub fn hash(&self) -> String {
        let mut hasher = DigestHasher::from(Sha1::new());

        self.etag.hash(&mut hasher);

        self.data.teachers.iter().for_each(|e| e.hash(&mut hasher));
        self.data.groups.iter().for_each(|e| e.hash(&mut hasher));

        hasher.finalize()
    }
}

/// Общие данные передаваемые в эндпоинты
pub struct AppState {
    pub downloader: Mutex<BasicXlsDownloader>,
    pub schedule: Mutex<Option<Schedule>>,
    pub database: Mutex<PgConnection>,
}

impl AppState {
    /// Получение объекта соединения с базой данных PostgreSQL
    pub fn connection(&self) -> MutexGuard<PgConnection> {
        self.database.lock().unwrap()
    }
}

/// Создание нового объекта web::Data<AppState>
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
