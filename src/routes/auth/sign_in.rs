use crate::database::driver;
use crate::database::models::User;
use crate::routes::auth::schema;
use crate::{AppState, utility};
use actix_web::{post, web};
use diesel::SaveChangesDsl;
use std::ops::DerefMut;
use web::Json;

#[post("/sign-in")]
pub async fn sign_in(
    data: Json<schema::sign_in::Request>,
    app_state: web::Data<AppState>,
) -> schema::sign_in::Response {
    use schema::sign_in::*;

    match driver::users::get_by_username(&app_state.database, data.username.clone()) {
        Ok(mut user) => match bcrypt::verify(&data.password, &user.password) {
            Ok(true) => {
                let mut lock = app_state.connection();
                let conn = lock.deref_mut();

                user.access_token = utility::jwt::encode(&user.id);

                user.save_changes::<User>(conn)
                    .expect("Failed to update user");

                Response::ok(&user)
            }
            Ok(false) | Err(_) => Response::err(ErrorCode::IncorrectCredentials),
        },

        Err(_) => Response::err(ErrorCode::IncorrectCredentials),
    }
}

#[cfg(test)]
mod tests {
    use crate::app_state::app_state;
    use crate::database::driver;
    use crate::database::models::{User, UserRole};
    use crate::routes::auth::schema;
    use crate::routes::auth::sign_in::sign_in;
    use crate::test_env::tests::{static_app_state, test_app, test_env};
    use crate::utility;
    use actix_http::StatusCode;
    use actix_web::dev::ServiceResponse;
    use actix_web::http::Method;
    use actix_web::test;
    use sha2::{Digest, Sha256};
    use std::fmt::Write;

    async fn sign_in_client(data: schema::sign_in::Request) -> ServiceResponse {
        let app = test_app(app_state(), sign_in).await;

        let req = test::TestRequest::with_uri("/sign-in")
            .method(Method::POST)
            .set_json(data)
            .to_request();

        test::call_service(&app, req).await
    }

    fn prepare(username: String) {
        let id = {
            let mut sha = Sha256::new();
            sha.update(&username);

            let result = sha.finalize();
            let bytes = &result[..12];

            let mut hex = String::new();
            for byte in bytes {
                write!(&mut hex, "{:02x}", byte).unwrap();
            }

            hex
        };

        test_env();

        let app_state = static_app_state();
        driver::users::insert_or_ignore(
            &app_state.database,
            &User {
                id: id.clone(),
                username,
                password: bcrypt::hash("example".to_string(), bcrypt::DEFAULT_COST).unwrap(),
                vk_id: None,
                access_token: utility::jwt::encode(&id),
                group: "ИС-214/23".to_string(),
                role: UserRole::Student,
                version: "1.0.0".to_string(),
            },
        )
        .unwrap();
    }

    #[actix_web::test]
    async fn sign_in_ok() {
        prepare("test::sign_in_ok".to_string());

        let resp = sign_in_client(schema::sign_in::Request {
            username: "test::sign_in_ok".to_string(),
            password: "example".to_string(),
        })
        .await;

        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[actix_web::test]
    async fn sign_in_err() {
        prepare("test::sign_in_err".to_string());

        let invalid_username = sign_in_client(schema::sign_in::Request {
            username: "test::sign_in_err::username".to_string(),
            password: "example".to_string(),
        })
        .await;

        assert_eq!(invalid_username.status(), StatusCode::NOT_ACCEPTABLE);

        let invalid_password = sign_in_client(schema::sign_in::Request {
            username: "test::sign_in_err".to_string(),
            password: "bad_password".to_string(),
        })
        .await;

        assert_eq!(invalid_password.status(), StatusCode::NOT_ACCEPTABLE);
    }
}
