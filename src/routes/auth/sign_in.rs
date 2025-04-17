use self::schema::*;
use crate::database::driver;
use crate::database::models::User;
use crate::routes::auth::shared::parse_vk_id;
use crate::routes::auth::sign_in::schema::SignInData::{Default, Vk};
use crate::routes::schema::user::UserResponse;
use crate::routes::schema::{IntoResponseAsError, ResponseError};
use crate::utility::mutex::MutexScope;
use crate::{AppState, utility};
use actix_web::{post, web};
use diesel::SaveChangesDsl;
use web::Json;

async fn sign_in_combined(
    data: SignInData,
    app_state: &web::Data<AppState>,
) -> Result<UserResponse, ErrorCode> {
    let user = match &data {
        Default(data) => driver::users::get_by_username(&app_state, &data.username),
        Vk(id) => driver::users::get_by_vk_id(&app_state, *id),
    };

    match user {
        Ok(mut user) => {
            if let Default(data) = data {
                match bcrypt::verify(&data.password, &user.password) {
                    Ok(result) => {
                        if !result {
                            return Err(ErrorCode::IncorrectCredentials);
                        }
                    }
                    Err(_) => {
                        return Err(ErrorCode::IncorrectCredentials);
                    }
                }
            }

            user.access_token = utility::jwt::encode(&user.id);

            app_state.database.scope(|conn| {
                user.save_changes::<User>(conn)
                    .expect("Failed to update user")
            });

            Ok(user.into())
        }

        Err(_) => Err(ErrorCode::IncorrectCredentials),
    }
}

#[utoipa::path(responses(
    (status = OK, body = UserResponse),
    (status = NOT_ACCEPTABLE, body = ResponseError<ErrorCode>)
))]
#[post("/sign-in")]
pub async fn sign_in(data: Json<Request>, app_state: web::Data<AppState>) -> ServiceResponse {
    sign_in_combined(Default(data.into_inner()), &app_state)
        .await
        .into()
}

#[utoipa::path(responses(
    (status = OK, body = UserResponse),
    (status = NOT_ACCEPTABLE, body = ResponseError<ErrorCode>)
))]
#[post("/sign-in-vk")]
pub async fn sign_in_vk(
    data_json: Json<vk::Request>,
    app_state: web::Data<AppState>,
) -> ServiceResponse {
    let data = data_json.into_inner();

    match parse_vk_id(&data.access_token, app_state.vk_id.client_id) {
        Ok(id) => sign_in_combined(Vk(id), &app_state).await.into(),
        Err(_) => ErrorCode::InvalidVkAccessToken.into_response(),
    }
}

mod schema {
    use crate::routes::schema::user::UserResponse;
    use actix_macros::{IntoResponseError, StatusCode};
    use serde::{Deserialize, Serialize};
    use utoipa::ToSchema;

    #[derive(Deserialize, Serialize, ToSchema)]
    #[schema(as = SignIn::Request)]
    pub struct Request {
        /// User name.
        #[schema(examples("n08i40k"))]
        pub username: String,

        /// Password.
        pub password: String,
    }

    pub mod vk {
        use serde::{Deserialize, Serialize};
        use utoipa::ToSchema;

        #[derive(Serialize, Deserialize, ToSchema)]
        #[serde(rename_all = "camelCase")]
        #[schema(as = SignInVk::Request)]
        pub struct Request {
            /// VK ID token.
            pub access_token: String,
        }
    }

    pub type ServiceResponse = crate::routes::schema::Response<UserResponse, ErrorCode>;

    #[derive(Serialize, ToSchema, Clone, IntoResponseError, StatusCode)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    #[schema(as = SignIn::ErrorCode)]
    #[status_code = "actix_web::http::StatusCode::NOT_ACCEPTABLE"]
    pub enum ErrorCode {
        /// Incorrect username or password.
        IncorrectCredentials,

        /// Invalid VK ID token.
        InvalidVkAccessToken,
    }

    /// Internal

    /// Type of authorization.
    pub enum SignInData {
        /// User and password name and password.
        Default(Request),

        /// Identifier of the attached account VK.
        Vk(i32),
    }
}

#[cfg(test)]
mod tests {
    use super::schema::*;
    use crate::database::driver;
    use crate::database::models::{User, UserRole};
    use crate::routes::auth::sign_in::sign_in;
    use crate::test_env::tests::{static_app_state, test_app_state, test_env};
    use crate::utility;
    use actix_test::test_app;
    use actix_web::dev::ServiceResponse;
    use actix_web::http::Method;
    use actix_web::http::StatusCode;
    use actix_web::test;
    use sha1::{Digest, Sha1};
    use std::fmt::Write;

    async fn sign_in_client(data: Request) -> ServiceResponse {
        let app = test_app(test_app_state().await, sign_in).await;

        let req = test::TestRequest::with_uri("/sign-in")
            .method(Method::POST)
            .set_json(data)
            .to_request();

        test::call_service(&app, req).await
    }

    async fn prepare(username: String) {
        let id = {
            let mut sha = Sha1::new();
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

        let app_state = static_app_state().await;
        driver::users::insert_or_ignore(
            &app_state,
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
        prepare("test::sign_in_ok".to_string()).await;

        let resp = sign_in_client(Request {
            username: "test::sign_in_ok".to_string(),
            password: "example".to_string(),
        })
        .await;

        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[actix_web::test]
    async fn sign_in_err() {
        prepare("test::sign_in_err".to_string()).await;

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
