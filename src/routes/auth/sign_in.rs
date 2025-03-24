use self::schema::*;
use crate::database::driver;
use crate::database::models::User;
use crate::routes::auth::shared::parse_vk_id;
use crate::routes::auth::sign_in::schema::ErrorCode;
use crate::routes::auth::sign_in::schema::SignInData::{Default, Vk};
use crate::{utility, AppState};
use actix_web::{post, web};
use diesel::SaveChangesDsl;
use std::ops::DerefMut;
use web::Json;

async fn sign_in(data: SignInData, app_state: &web::Data<AppState>) -> Response {
    let user = match &data {
        Default(data) => driver::users::get_by_username(&app_state.database, &data.username),
        Vk(id) => driver::users::get_by_vk_id(&app_state.database, *id),
    };

    match user {
        Ok(mut user) => {
            if let Default(data) = data {
                match bcrypt::verify(&data.password, &user.password) {
                    Ok(result) => {
                        if !result {
                            return Response::err(ErrorCode::IncorrectCredentials);
                        }
                    }
                    Err(_) => {
                        return Response::err(ErrorCode::IncorrectCredentials);
                    }
                }
            }

            let mut lock = app_state.connection();
            let conn = lock.deref_mut();

            user.access_token = utility::jwt::encode(&user.id);

            user.save_changes::<User>(conn)
                .expect("Failed to update user");

            Response::ok(&user)
        }

        Err(_) => Response::err(ErrorCode::IncorrectCredentials),
    }
}

#[post("/sign-in")]
pub async fn sign_in_default(data: Json<Request>, app_state: web::Data<AppState>) -> Response {
    sign_in(Default(data.into_inner()), &app_state).await
}

#[post("/sign-in-vk")]
pub async fn sign_in_vk(data_json: Json<vk::Request>, app_state: web::Data<AppState>) -> Response {
    let data = data_json.into_inner();

    match parse_vk_id(&data.access_token) {
        Ok(id) => sign_in(Vk(id), &app_state).await,
        Err(_) => Response::err(ErrorCode::InvalidVkAccessToken),
    }
}

mod schema {
    use crate::database::models::User;
    use crate::routes::schema::{user, ErrorToHttpCode, IResponse};
    use actix_web::http::StatusCode;
    use serde::{Deserialize, Serialize};

    #[derive(Deserialize, Serialize)]
    pub struct Request {
        pub username: String,
        pub password: String,
    }

    pub mod vk {
        use serde::{Deserialize, Serialize};

        #[derive(Serialize, Deserialize)]
        #[serde(rename_all = "camelCase")]
        pub struct Request {
            pub access_token: String,
        }
    }

    pub type Response = IResponse<user::ResponseOk, ResponseErr>;

    #[derive(Serialize)]
    pub struct ResponseErr {
        code: ErrorCode,
    }

    #[derive(Serialize)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum ErrorCode {
        IncorrectCredentials,
        InvalidVkAccessToken,
    }

    pub trait ResponseExt {
        fn ok(user: &User) -> Self;
        fn err(code: ErrorCode) -> Response;
    }

    impl ResponseExt for Response {
        fn ok(user: &User) -> Self {
            IResponse(Ok(user::ResponseOk::from_user(&user)))
        }

        fn err(code: ErrorCode) -> Response {
            IResponse(Err(ResponseErr { code }))
        }
    }

    impl ErrorToHttpCode for ResponseErr {
        fn to_http_status_code(&self) -> StatusCode {
            StatusCode::NOT_ACCEPTABLE
        }
    }

    /// Internal

    pub enum SignInData {
        Default(Request),
        Vk(i32),
    }
}

#[cfg(test)]
mod tests {
    use super::schema::*;
    use crate::database::driver;
    use crate::database::models::{User, UserRole};
    use crate::routes::auth::sign_in::sign_in_default;
    use crate::test_env::tests::{static_app_state, test_app, test_app_state, test_env};
    use crate::utility;
    use actix_http::StatusCode;
    use actix_web::dev::ServiceResponse;
    use actix_web::http::Method;
    use actix_web::test;
    use sha2::{Digest, Sha256};
    use std::fmt::Write;

    async fn sign_in_client(data: Request) -> ServiceResponse {
        let app = test_app(test_app_state(), sign_in_default).await;

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

        let resp = sign_in_client(Request {
            username: "test::sign_in_ok".to_string(),
            password: "example".to_string(),
        })
        .await;

        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[actix_web::test]
    async fn sign_in_err() {
        prepare("test::sign_in_err".to_string());

        let invalid_username = sign_in_client(Request {
            username: "test::sign_in_err::username".to_string(),
            password: "example".to_string(),
        })
        .await;

        assert_eq!(invalid_username.status(), StatusCode::NOT_ACCEPTABLE);

        let invalid_password = sign_in_client(Request {
            username: "test::sign_in_err".to_string(),
            password: "bad_password".to_string(),
        })
        .await;

        assert_eq!(invalid_password.status(), StatusCode::NOT_ACCEPTABLE);
    }
}
