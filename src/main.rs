use crate::app_state::{AppState, app_state};
use crate::routes::auth::sign_in::sign_in;
use actix_web::{App, HttpServer, web};
use dotenvy::dotenv;

mod app_state;
mod database;
mod routes;

#[cfg(test)]
mod test_env;

mod utility;
mod xls_downloader;

#[actix_web::main]
async fn main() {
    dotenv().ok();

    HttpServer::new(move || {
        let schedule_scope = web::scope("/auth").service(sign_in);
        let api_scope = web::scope("/api/v1").service(schedule_scope);

        App::new().app_data(move || app_state()).service(api_scope)
    })
    .bind(("127.0.0.1", 8080))
    .unwrap()
    .run()
    .await
    .unwrap();
}
