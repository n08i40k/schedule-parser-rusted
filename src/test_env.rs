#[cfg(test)]
pub(crate) mod tests {
    use crate::app_state::{app_state, AppState, Schedule};
    use crate::parser::tests::test_result;
    use actix_web::{web};
    use std::sync::LazyLock;

    pub fn test_env() {
        dotenvy::from_path(".env.test").expect("Failed to load test environment file");
    }

    pub fn test_app_state() -> web::Data<AppState> {
        let state = app_state();
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

    pub fn static_app_state() -> web::Data<AppState> {
        static STATE: LazyLock<web::Data<AppState>> = LazyLock::new(|| test_app_state());

        STATE.clone()
    }
}
