#[cfg(test)]
pub(crate) mod tests {
    use crate::app_state::{AppState, Schedule, app_state};
    use crate::parser::tests::test_result;
    use actix_web::web;
    use tokio::sync::OnceCell;

    pub fn test_env() {
        dotenvy::from_path(".env.test").expect("Failed to load test environment file");
    }

    pub async fn test_app_state() -> web::Data<AppState> {
        let state = app_state().await;
        let mut schedule_lock = state.schedule.lock().unwrap();

        *schedule_lock = Some(Schedule {
            etag: "".to_string(),
            fetched_at: Default::default(),
            updated_at: Default::default(),
            parsed_at: Default::default(),
            data: test_result().unwrap(),
        });

        state.clone()
    }

    pub async fn static_app_state() -> web::Data<AppState> {
        static STATE: OnceCell<web::Data<AppState>> = OnceCell::const_new();

        STATE.get_or_init(|| test_app_state()).await.clone()
    }
}
