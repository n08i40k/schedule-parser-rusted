mod env;
mod fcm_client;

pub use crate::state::env::AppEnv;
use crate::state::fcm_client::FCMClientData;
use actix_web::web;
use diesel::{Connection, PgConnection};
use firebase_messaging_rs::FCMClient;
use providers::base::{ScheduleProvider, ScheduleSnapshot};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::{Mutex, MutexGuard};
use tokio_util::sync::CancellationToken;

/// Common data provided to endpoints.
pub struct AppState {
    cancel_token: CancellationToken,
    database: Mutex<PgConnection>,
    providers: HashMap<String, Arc<dyn ScheduleProvider>>,
    env: AppEnv,
    fcm_client: Option<Mutex<FCMClient>>,
}

impl AppState {
    pub async fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

        let env = AppEnv::default();
        let providers: HashMap<String, Arc<dyn ScheduleProvider>> = HashMap::from([(
            "eng_polytechnic".to_string(),
            providers::EngelsPolytechnicProvider::new({
                #[cfg(test)]
                {
                    providers::EngelsPolytechnicUpdateSource::Prepared(ScheduleSnapshot {
                        url: "".to_string(),
                        fetched_at: chrono::DateTime::default(),
                        updated_at: chrono::DateTime::default(),
                        data: providers::test_utils::engels_polytechnic::test_result().unwrap(),
                    })
                }

                #[cfg(not(test))]
                {
                    if let Some(url) = &env.schedule.url {
                        providers::EngelsPolytechnicUpdateSource::Url(url.clone())
                    } else {
                        providers::EngelsPolytechnicUpdateSource::GrabFromSite {
                            yandex_api_key: env.yandex_cloud.api_key.clone(),
                            yandex_func_id: env.yandex_cloud.func_id.clone(),
                        }
                    }
                }
            })
            .await?,
        )]);

        let this = Self {
            cancel_token: CancellationToken::new(),
            database: Mutex::new(
                PgConnection::establish(&database_url)
                    .unwrap_or_else(|_| panic!("Error connecting to {}", database_url)),
            ),
            env,
            providers,
            fcm_client: FCMClientData::new().await,
        };

        if this.env.schedule.auto_update {
            for (_, provider) in &this.providers {
                let provider = provider.clone();
                let cancel_token = this.cancel_token.clone();

                tokio::spawn(async move { provider.start_auto_update_task(cancel_token).await });
            }
        }

        Ok(this)
    }

    pub async fn get_schedule_snapshot(&'_ self, provider: &str) -> Option<Arc<ScheduleSnapshot>> {
        if let Some(provider) = self.providers.get(provider) {
            return Some(provider.get_schedule().await);
        }

        None
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
pub async fn new_app_state() -> Result<web::Data<AppState>, Box<dyn std::error::Error>> {
    Ok(web::Data::new(AppState::new().await?))
}
