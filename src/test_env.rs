#[cfg(test)]
pub(crate) mod tests {
    use crate::app_state::{AppState, app_state};
    use actix_web::dev::{HttpServiceFactory, Service, ServiceResponse};
    use actix_web::{App, test, web};
    use std::sync::LazyLock;

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

    pub fn static_app_state() -> web::Data<AppState> {
        static STATE: LazyLock<web::Data<AppState>> = LazyLock::new(|| app_state());

        STATE.clone()
    }
}
