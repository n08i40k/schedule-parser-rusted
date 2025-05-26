#[cfg(test)]
pub(crate) mod tests {
    use crate::app_state::{AppState, Schedule, app_state};
    use schedule_parser::test_utils::test_result;
    use crate::utility::mutex::MutexScope;
    use actix_web::web;
    use std::default::Default;
    use tokio::sync::OnceCell;

    pub fn test_env() {
        dotenvy::from_path(".env.test").expect("Failed to load test environment file");
    }

    pub enum TestScheduleType {
        None,
        Local,
    }

    pub struct TestAppStateParams {
        pub schedule: TestScheduleType,
    }

    impl Default for TestAppStateParams {
        fn default() -> Self {
            Self {
                schedule: TestScheduleType::None,
            }
        }
    }

    pub async fn test_app_state(params: TestAppStateParams) -> web::Data<AppState> {
        let state = app_state().await;

        state.schedule.scope(|schedule| {
            *schedule = match params.schedule {
                TestScheduleType::None => None,
                TestScheduleType::Local => Some(Schedule {
                    etag: "".to_string(),
                    fetched_at: Default::default(),
                    updated_at: Default::default(),
                    parsed_at: Default::default(),
                    data: test_result().unwrap(),
                }),
            }
        });

        state.clone()
    }

    pub async fn static_app_state() -> web::Data<AppState> {
        static STATE: OnceCell<web::Data<AppState>> = OnceCell::const_new();

        STATE
            .get_or_init(|| test_app_state(Default::default()))
            .await
            .clone()
    }
}
