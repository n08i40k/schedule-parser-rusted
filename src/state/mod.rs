mod env;

pub use crate::state::env::AppEnv;
use actix_web::web;
use database::migration::{Migrator, MigratorTrait};
use database::sea_orm::{ConnectOptions, Database, DatabaseConnection};
use providers::base::{ScheduleProvider, ScheduleSnapshot};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio_util::sync::CancellationToken;

/// Common data provided to endpoints.
pub struct AppState {
    cancel_token: CancellationToken,
    database: DatabaseConnection,
    providers: HashMap<String, Arc<dyn ScheduleProvider>>,
    env: AppEnv,
}

impl AppState {
    pub async fn new(
        database: Option<DatabaseConnection>,
    ) -> Result<Self, Box<dyn std::error::Error>> {
        let env = AppEnv::default();
        let providers: HashMap<String, Arc<dyn ScheduleProvider>> = HashMap::from([(
            "eng_polytechnic".to_string(),
            providers::EngelsPolytechnicProvider::get({
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
            database: if let Some(database) = database {
                database
            } else {
                let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

                let mut opt = ConnectOptions::new(database_url.clone());

                opt.max_connections(4)
                    .min_connections(2)
                    .connect_timeout(Duration::from_secs(10))
                    .idle_timeout(Duration::from_secs(8))
                    .sqlx_logging(true);

                let database = Database::connect(opt)
                    .await
                    .unwrap_or_else(|_| panic!("Error connecting to {}", database_url));

                Migrator::up(&database, None)
                    .await
                    .expect("Failed to run database migrations");

                database
            },
            env,
            providers,
        };

        if this.env.schedule.auto_update {
            for provider in this.providers.values() {
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

    pub fn get_database(&'_ self) -> &DatabaseConnection {
        &self.database
    }

    pub fn get_env(&self) -> &AppEnv {
        &self.env
    }
}

/// Create a new object web::Data<AppState>.
pub async fn new_app_state(
    database: Option<DatabaseConnection>,
) -> Result<web::Data<AppState>, Box<dyn std::error::Error>> {
    Ok(web::Data::new(AppState::new(database).await?))
}
