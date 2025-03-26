use actix_web::dev::{HttpServiceFactory, Service, ServiceResponse};
use actix_web::{App, test, web};

pub async fn test_app<F, A: 'static>(
    app_state: web::Data<A>,
    factory: F,
) -> impl Service<actix_http::Request, Response = ServiceResponse, Error = actix_web::Error>
where
    F: HttpServiceFactory + 'static,
{
    test::init_service(App::new().app_data(app_state).service(factory)).await
}
