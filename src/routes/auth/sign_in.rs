use self::schema::*;
use crate::routes::auth::shared::parse_vk_id;
use crate::routes::auth::sign_in::schema::SignInData::{Default, VkOAuth};
use crate::routes::schema::user::UserResponse;
use crate::routes::schema::ResponseError;
use crate::{utility, AppState};
use actix_web::{post, web};
use database::query::Query;
use web::Json;
use database::entity::UserType;

async fn sign_in_combined(
    data: SignInData,
    app_state: &web::Data<AppState>,
) -> Result<UserResponse, ErrorCode> {
    let db = app_state.get_database();

    let user = match &data {
        Default(data) => Query::find_user_by_username(db, &data.username).await,
        VkOAuth(id) => Query::find_user_by_vk_id(db, *id).await,
    }
    .ok()
    .flatten();

    match user {
        Some(user) => {
            if let Default(data) = data {
                if user.password.is_none() {
                    return Err(ErrorCode::IncorrectCredentials);
                }

                match bcrypt::verify(&data.password, user.password.as_ref().unwrap()) {
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

            let access_token = utility::jwt::encode(UserType::Default, &user.id);
            Ok(UserResponse::from_user_with_token(user, access_token))
        }

        None => Err(ErrorCode::IncorrectCredentials),
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

    match parse_vk_id(&data.access_token, app_state.get_env().vk_id.client_id) {
        Ok(id) => sign_in_combined(VkOAuth(id), &app_state).await,
        Err(_) => Err(ErrorCode::InvalidVkAccessToken),
    }
    .into()
}

mod schema {
    use crate::routes::schema::user::UserResponse;
    use actix_macros::ErrResponse;
    use derive_more::Display;
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

    #[derive(Clone, Serialize, Display, ToSchema, ErrResponse)]
    #[schema(as = SignIn::ErrorCode)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    #[status_code = "actix_web::http::StatusCode::NOT_ACCEPTABLE"]
    pub enum ErrorCode {
        /// Incorrect username or password.
        #[display("Incorrect username or password.")]
        IncorrectCredentials,

        /// Invalid VK ID token.
        #[display("Invalid VK ID token.")]
        InvalidVkAccessToken,
    }

    /// Type of authorization.
    pub enum SignInData {
        /// User and password name and password.
        Default(Request),

        /// Identifier of the attached account VK.
        VkOAuth(i32),
    }
}

#[cfg(test)]
mod tests {
    use super::schema::*;
    use crate::routes::auth::sign_in::sign_in;
    use crate::test_env::tests::{static_app_state, test_app_state, test_env};
    use actix_test::test_app;
    use actix_web::dev::ServiceResponse;
    use actix_web::http::Method;
    use actix_web::http::StatusCode;
    use actix_web::test;
    use database::entity::sea_orm_active_enums::UserRole;
    use database::entity::ActiveUser;
    use database::sea_orm::{ActiveModelTrait, Set};
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

        let active_user = ActiveUser {
            id: Set(id.clone()),
            username: Set(username),
            password: Set(Some(bcrypt::hash("example", bcrypt::DEFAULT_COST).unwrap())),
            vk_id: Set(None),
            telegram_id: Set(None),
            group: Set(Some("ИС-214/23".to_string())),
            role: Set(UserRole::Student),
            android_version: Set(None),
        };

        active_user
            .save(app_state.get_database())
            .await
            .expect("Failed to save user");
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
