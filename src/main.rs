use crate::app_state::{app_state, AppState};
use crate::middlewares::authorization::Authorization;
use crate::routes::auth::sign_in::{sign_in_default, sign_in_vk};
use crate::routes::auth::sign_up::{sign_up_default, sign_up_vk};
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

    HttpServer::new(move || {
        let auth_scope = utoipa_actix_web::scope("/auth")
            .service(sign_in_default)
            .service(sign_in_vk)
            .service(sign_up_default)
            .service(sign_up_vk);

        let users_scope = utoipa_actix_web::scope("/users")
            .wrap(Authorization)
            .service(me);

        let api_scope = utoipa_actix_web::scope("/api/v1")
            .service(auth_scope)
            .service(users_scope);

        let (app, api) = App::new()
            .into_utoipa_app()
            .app_data(app_state())
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
    .bind(("0.0.0.0", 8080))
    .unwrap()
    .run()
    .await
    .unwrap();
}
