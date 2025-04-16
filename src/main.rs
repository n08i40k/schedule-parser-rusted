use crate::app_state::{AppState, app_state};
use crate::middlewares::authorization::JWTAuthorization;
use crate::middlewares::content_type::ContentTypeBootstrap;
use actix_web::dev::{ServiceFactory, ServiceRequest};
use actix_web::{App, Error, HttpServer};
use dotenvy::dotenv;
use utoipa_actix_web::AppExt;
use utoipa_actix_web::scope::Scope;
use utoipa_rapidoc::RapiDoc;

mod app_state;

mod database;

mod parser;
mod xls_downloader;

mod extractors;
mod middlewares;
mod routes;

mod utility;

mod test_env;

pub fn get_api_scope<
    I: Into<Scope<T>>,
    T: ServiceFactory<ServiceRequest, Config = (), Error = Error, InitError = ()>,
>(
    scope: I,
) -> Scope<T> {
    let auth_scope = utoipa_actix_web::scope("/auth")
        .service(routes::auth::sign_in)
        .service(routes::auth::sign_in_vk)
        .service(routes::auth::sign_up)
        .service(routes::auth::sign_up_vk);

    let users_scope = utoipa_actix_web::scope("/users")
        .wrap(JWTAuthorization::default())
        .service(routes::users::change_group)
        .service(routes::users::change_username)
        .service(routes::users::me);

    let schedule_scope = utoipa_actix_web::scope("/schedule")
        .wrap(JWTAuthorization {
            ignore: &["/group-names", "/teacher-names"],
        })
        .service(routes::schedule::schedule)
        .service(routes::schedule::update_download_url)
        .service(routes::schedule::cache_status)
        .service(routes::schedule::group)
        .service(routes::schedule::group_names)
        .service(routes::schedule::teacher)
        .service(routes::schedule::teacher_names);

    let fcm_scope = utoipa_actix_web::scope("/fcm")
        .wrap(JWTAuthorization::default())
        .service(routes::fcm::update_callback)
        .service(routes::fcm::set_token);

    let vk_id_scope = utoipa_actix_web::scope("/vkid") //
        .service(routes::vk_id::oauth);

    utoipa_actix_web::scope(scope)
        .service(auth_scope)
        .service(users_scope)
        .service(schedule_scope)
        .service(fcm_scope)
        .service(vk_id_scope)
}

#[actix_web::main]
async fn main() {
    dotenv().ok();

    unsafe { std::env::set_var("RUST_LOG", "debug") };
    env_logger::init();

    let app_state = app_state().await;

    HttpServer::new(move || {
        let (app, api) = App::new()
            .into_utoipa_app()
            .app_data(app_state.clone())
            .service(get_api_scope("/api/v1").wrap(ContentTypeBootstrap))
            .split_for_parts();

        let rapidoc_service = RapiDoc::with_openapi("/api-docs-json", api).path("/api-docs");

        // Because CORS error on non-localhost
        let patched_rapidoc_html = rapidoc_service.to_html().replace(
            "https://unpkg.com/rapidoc/dist/rapidoc-min.js",
            "https://cdn.jsdelivr.net/npm/rapidoc/dist/rapidoc-min.min.js",
        );

        app.service(rapidoc_service.custom_html(patched_rapidoc_html))
    })
    .workers(4)
    .bind(("0.0.0.0", 5050))
    .unwrap()
    .run()
    .await
    .unwrap();
}
