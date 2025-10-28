use crate::middlewares::authorization::{JWTAuthorizationBuilder, ServiceConfig};
use crate::middlewares::content_type::ContentTypeBootstrap;
use crate::state::{new_app_state, AppState};
use actix_web::dev::{ServiceFactory, ServiceRequest};
use actix_web::{App, Error, HttpServer};
use database::entity::sea_orm_active_enums::UserRole;
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
    let admin_scope = {
        let service_user_scope =
            utoipa_actix_web::scope("/service-users").service(routes::admin::service_users::create);

        utoipa_actix_web::scope("/admin")
            .wrap(
                JWTAuthorizationBuilder::new()
                    .with_default(Some(ServiceConfig {
                        allow_service: false,
                        user_roles: Some(&[UserRole::Admin]),
                    }))
                    .build(),
            )
            .service(service_user_scope)
    };

    let auth_scope = utoipa_actix_web::scope("/auth")
        .service(routes::auth::sign_in)
        .service(routes::auth::sign_in_vk)
        .service(routes::auth::sign_up)
        .service(routes::auth::sign_up_vk);

    let users_scope = utoipa_actix_web::scope("/users")
        .wrap(
            JWTAuthorizationBuilder::new()
                .add_paths(
                    ["/by/id/{id}", "/by/telegram-id/{id}"],
                    Some(ServiceConfig {
                        allow_service: true,
                        user_roles: Some(&[UserRole::Admin]),
                    }),
                )
                .build(),
        )
        .service(
            utoipa_actix_web::scope("/by")
                .service(routes::users::by::by_id)
                .service(routes::users::by::by_telegram_id),
        )
        .service(routes::users::change_group)
        .service(routes::users::change_username)
        .service(routes::users::me);

    let schedule_scope = utoipa_actix_web::scope("/schedule")
        .wrap(
            JWTAuthorizationBuilder::new()
                .with_default(Some(ServiceConfig {
                    allow_service: true,
                    user_roles: None,
                }))
                .add_paths(["/group-names", "/teacher-names"], None)
                .add_paths(
                    ["/"],
                    Some(ServiceConfig {
                        allow_service: true,
                        user_roles: Some(&[UserRole::Admin]),
                    }),
                )
                .add_paths(
                    ["/group"],
                    Some(ServiceConfig {
                        allow_service: false,
                        user_roles: None,
                    }),
                )
                .build(),
        )
        .service(routes::schedule::cache_status)
        .service(routes::schedule::schedule)
        .service(routes::schedule::group)
        .service(routes::schedule::group_by_name)
        .service(routes::schedule::group_names)
        .service(routes::schedule::teacher)
        .service(routes::schedule::teacher_names);

    let flow_scope = utoipa_actix_web::scope("/flow")
        .wrap(
            JWTAuthorizationBuilder::new()
                .add_paths(["/telegram-auth"], None)
                .build(),
        )
        .service(routes::flow::telegram_auth)
        .service(routes::flow::telegram_complete);

    let vk_id_scope = utoipa_actix_web::scope("/vkid") //
        .service(routes::vk_id::oauth);

    utoipa_actix_web::scope(scope)
        .service(admin_scope)
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
