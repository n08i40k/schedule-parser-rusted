use crate::app_state::{AppState, app_state};
use crate::routes::auth::sign_in::{sign_in_default, sign_in_vk};
use crate::routes::auth::sign_up::{sign_up_default, sign_up_vk};
use actix_web::{App, HttpServer, web};
use dotenvy::dotenv;

mod app_state;
mod database;
mod routes;

mod test_env;

mod utility;
mod xls_downloader;

mod parser;

#[actix_web::main]
async fn main() {
    dotenv().ok();

    HttpServer::new(move || {
        let auth_scope = web::scope("/auth")
            .service(sign_in_default)
            .service(sign_in_vk)
            .service(sign_up_default)
            .service(sign_up_vk);
        let api_scope = web::scope("/api/v1").service(auth_scope);

        App::new().app_data(move || app_state()).service(api_scope)
    })
    .bind(("127.0.0.1", 8080))
    .unwrap()
    .run()
    .await
    .unwrap();
}
