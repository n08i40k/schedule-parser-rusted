#[cfg(test)]
pub(crate) mod tests {
    use crate::app_state::{app_state, AppState, Schedule};
    use actix_web::dev::{HttpServiceFactory, Service, ServiceResponse};
    use actix_web::{test, web, App};
    use std::sync::LazyLock;
    use crate::parser::tests::test_result;

    pub fn test_env() {
        dotenvy::from_path(".env.test").expect("Failed to load test environment file");
    }

    pub async fn test_app<F>(
        app_state: web::Data<AppState>,
        factory: F,
    ) -> impl Service<actix_http::Request, Response = ServiceResponse, Error = actix_web::Error>
    where
        F: HttpServiceFactory + 'static,
    {
        test::init_service(App::new().app_data(app_state).service(factory)).await
    }

    pub fn test_app_state() -> web::Data<AppState> {
        let state = app_state();
        let mut schedule_lock = state.schedule.lock().unwrap();

        *schedule_lock = Some(Schedule {
            etag: "".to_string(),
            updated_at: Default::default(),
            parsed_at: Default::default(),
            data: test_result(),
        });

        state.clone()
    }

    pub fn static_app_state() -> web::Data<AppState> {
        static STATE: LazyLock<web::Data<AppState>> = LazyLock::new(|| test_app_state());

        STATE.clone()
    }
}
