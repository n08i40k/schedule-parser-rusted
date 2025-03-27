use crate::app_state::{app_state, AppState};
use crate::middlewares::authorization::Authorization;
use crate::routes::auth::sign_in::{sign_in_default, sign_in_vk};
use crate::routes::auth::sign_up::{sign_up_default, sign_up_vk};
use crate::routes::users::me::me;
use actix_web::{web, App, HttpServer};
use dotenvy::dotenv;

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
        let auth_scope = web::scope("/auth")
            .service(sign_in_default)
            .service(sign_in_vk)
            .service(sign_up_default)
            .service(sign_up_vk);

        let users_scope = web::scope("/users")
            .wrap(Authorization)
            .service(me);

        let api_scope = web::scope("/api/v1")
            .service(auth_scope)
            .service(users_scope);

        App::new().app_data(app_state()).service(api_scope)
    })
    .bind(("127.0.0.1", 8080))
    .unwrap()
    .run()
    .await
    .unwrap();
}
