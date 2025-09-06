use crate::middlewares::authorization::JWTAuthorization;
use crate::middlewares::content_type::ContentTypeBootstrap;
use crate::state::{new_app_state, AppState};
use actix_web::dev::{ServiceFactory, ServiceRequest};
use actix_web::{App, Error, HttpServer};
use dotenvy::dotenv;
use log::info;
use std::io;
use utoipa_actix_web::scope::Scope;
use utoipa_actix_web::AppExt;
use utoipa_rapidoc::RapiDoc;

mod state;

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
        .service(routes::schedule::cache_status)
        .service(routes::schedule::group)
        .service(routes::schedule::group_names)
        .service(routes::schedule::teacher)
        .service(routes::schedule::teacher_names);

    let flow_scope = utoipa_actix_web::scope("/flow")
        .wrap(JWTAuthorization {
            ignore: &["/telegram-auth"],
        })
        .service(routes::flow::telegram_auth)
        .service(routes::flow::telegram_complete);

    let vk_id_scope = utoipa_actix_web::scope("/vkid") //
        .service(routes::vk_id::oauth);

    utoipa_actix_web::scope(scope)
        .service(auth_scope)
        .service(users_scope)
        .service(schedule_scope)
        .service(flow_scope)
        .service(vk_id_scope)
}

async fn async_main() -> io::Result<()> {
    info!("Запуск сервера...");

    let app_state = new_app_state(None).await.unwrap();

    HttpServer::new(move || {
        let (app, api) = App::new()
            .into_utoipa_app()
            .app_data(app_state.clone())
            .service(
                get_api_scope("/api/v1")
                    .wrap(sentry_actix::Sentry::new())
                    .wrap(ContentTypeBootstrap),
            )
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
    .bind(("0.0.0.0", 5050))?
    .run()
    .await
}

fn main() -> io::Result<()> {
    let _guard = sentry::init((
        "https://9c33db76e89984b3f009b28a9f4b5954@sentry.n08i40k.ru/8",
        sentry::ClientOptions {
            release: sentry::release_name!(),
            send_default_pii: true,
            ..Default::default()
        },
    ));

    let _ = dotenv();

    env_logger::init();

    actix_web::rt::System::new().block_on(async { async_main().await })?;

    Ok(())
}
