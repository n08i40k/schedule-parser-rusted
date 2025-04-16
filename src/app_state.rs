use crate::parser::schema::ParseResult;
use crate::utility::hasher::DigestHasher;
use crate::xls_downloader::basic_impl::BasicXlsDownloader;
use actix_web::web;
use chrono::{DateTime, Utc};
use diesel::{Connection, PgConnection};
use firebase_messaging_rs::FCMClient;
use sha1::{Digest, Sha1};
use std::env;
use std::hash::Hash;
use std::sync::Mutex;

#[derive(Clone)]
pub struct Schedule {
    pub etag: String,
    pub fetched_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub parsed_at: DateTime<Utc>,
    pub data: ParseResult,
}

#[derive(Clone)]
pub struct VkId {
    pub client_id: i32,
    pub redirect_url: String,
}

impl VkId {
    pub fn new() -> Self {
        Self {
            client_id: env::var("VKID_CLIENT_ID")
                .expect("VKID_CLIENT_ID must be set")
                .parse()
                .expect("VKID_CLIENT_ID must be integer"),
            redirect_url: env::var("VKID_REDIRECT_URI").expect("VKID_REDIRECT_URI must be set"),
        }
    }
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

/// Common data provided to endpoints.
pub struct AppState {
    pub downloader: Mutex<BasicXlsDownloader>,
    pub schedule: Mutex<Option<Schedule>>,
    pub database: Mutex<PgConnection>,
    pub vk_id: VkId,
    pub fcm_client: Option<Mutex<FCMClient>>, // в рантайме не меняется, так что опционален мьютекс, а не данные в нём.
}

impl AppState {
    pub async fn new() -> Self {
        let database_url = env::var("DATABASE_URL").expect("DATABASE_URL must be set");

        Self {
            downloader: Mutex::new(BasicXlsDownloader::new()),
            schedule: Mutex::new(None),
            database: Mutex::new(
                PgConnection::establish(&database_url)
                    .unwrap_or_else(|_| panic!("Error connecting to {}", database_url)),
            ),
            vk_id: VkId::new(),
            fcm_client: if env::var("GOOGLE_APPLICATION_CREDENTIALS").is_ok() {
                Some(Mutex::new(
                    FCMClient::new().await.expect("FCM client must be created"),
                ))
            } else {
                None
            },
        }
    }
}

/// Create a new object web::Data<AppState>.
pub async fn app_state() -> web::Data<AppState> {
    web::Data::new(AppState::new().await)
}
