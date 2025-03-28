use crate::app_state::{AppState, app_state};
use crate::middlewares::authorization::JWTAuthorization;
use crate::routes::auth::sign_in::{sign_in_default, sign_in_vk};
use crate::routes::auth::sign_up::{sign_up_default, sign_up_vk};
use crate::routes::schedule::get_cache_status::get_cache_status;
use crate::routes::schedule::get_group::get_group;
use crate::routes::schedule::get_group_names::get_group_names;
use crate::routes::schedule::get_schedule::get_schedule;
use crate::routes::schedule::get_teacher::get_teacher;
use crate::routes::schedule::get_teacher_names::get_teacher_names;
use crate::routes::schedule::update_download_url::update_download_url;
use crate::routes::users::me::me;
use actix_web::{App, HttpServer};
use dotenvy::dotenv;
use utoipa_actix_web::AppExt;
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

#[actix_web::main]
async fn main() {
    dotenv().ok();

    unsafe { std::env::set_var("RUST_LOG", "debug") };
    env_logger::init();

    let app_state = app_state();

    HttpServer::new(move || {
        let auth_scope = utoipa_actix_web::scope("/auth")
            .service(sign_in_default)
            .service(sign_in_vk)
            .service(sign_up_default)
            .service(sign_up_vk);

        let users_scope = utoipa_actix_web::scope("/users")
            .wrap(JWTAuthorization)
            .service(me);

        let schedule_scope = utoipa_actix_web::scope("/schedule")
            .wrap(JWTAuthorization)
            .service(get_schedule)
            .service(update_download_url)
            .service(get_cache_status)
            .service(get_group)
            .service(get_group_names)
            .service(get_teacher)
            .service(get_teacher_names);

        let api_scope = utoipa_actix_web::scope("/api/v1")
            .service(auth_scope)
            .service(users_scope)
            .service(schedule_scope);

        let (app, api) = App::new()
            .into_utoipa_app()
            .app_data(app_state.clone())
            .service(api_scope)
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
    .bind(("0.0.0.0", 8080))
    .unwrap()
    .run()
    .await
    .unwrap();
}
