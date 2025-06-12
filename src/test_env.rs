#[cfg(test)]
pub(crate) mod tests {
    use crate::state::{AppState, ScheduleSnapshot, new_app_state};
    use actix_web::web;
    use log::info;
    use schedule_parser::test_utils::test_result;
    use std::default::Default;
    use tokio::sync::OnceCell;

    pub fn test_env() {
        info!("Loading test environment file...");
        dotenvy::from_filename(".env.test.local")
            .or_else(|_| dotenvy::from_filename(".env.test"))
            .expect("Failed to load test environment file");
    }

    pub async fn test_app_state() -> web::Data<AppState> {
        let state = new_app_state().await.unwrap();

        state.get_schedule().await.snapshot.write(ScheduleSnapshot {
            fetched_at: Default::default(),
            updated_at: Default::default(),
            url: "".to_string(),
            data: test_result().unwrap(),
        });

        state.clone()
    }

    pub async fn static_app_state() -> web::Data<AppState> {
        static STATE: OnceCell<web::Data<AppState>> = OnceCell::const_new();

        STATE.get_or_init(|| test_app_state()).await.clone()
    }
}
