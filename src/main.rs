use crate::routes::auth::sign_in::sign_in;
use crate::xls_downloader::basic_impl::BasicXlsDownloader;
use actix_web::{App, HttpServer, web};
use chrono::{DateTime, Utc};
use diesel::{Connection, PgConnection};
use dotenvy::dotenv;
use schedule_parser::schema::ScheduleEntity;
use std::collections::HashMap;
use std::env;
use std::sync::{Mutex, MutexGuard};

mod database;
mod routes;
mod utility;
mod xls_downloader;

pub struct AppState {
    downloader: Mutex<BasicXlsDownloader>,
    schedule: Mutex<
        Option<(
            String,
            DateTime<Utc>,
            (
                HashMap<String, ScheduleEntity>,
                HashMap<String, ScheduleEntity>,
            ),
        )>,
    >,
    database: Mutex<PgConnection>,
}

impl AppState {
    pub fn connection(&self) -> MutexGuard<PgConnection> {
        self.database.lock().unwrap()
    }
}

#[actix_web::main]
async fn main() {
    dotenv().ok();

    let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    let data = web::Data::new(AppState {
        downloader: Mutex::new(BasicXlsDownloader::new()),
        schedule: Mutex::new(None),
        database: Mutex::new(
            PgConnection::establish(&database_url)
                .unwrap_or_else(|_| panic!("Error connecting to {}", database_url)),
        ),
    });

    HttpServer::new(move || {
        let schedule_scope = web::scope("/auth").service(sign_in);
        let api_scope = web::scope("/api/v1").service(schedule_scope);

        App::new().app_data(data.clone()).service(api_scope)
    })
    .bind(("127.0.0.1", 8080))
    .unwrap()
    .run()
    .await
    .unwrap();
}
