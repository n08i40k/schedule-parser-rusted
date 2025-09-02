use self::schema::*;
use crate::AppState;
use crate::database::driver;
use crate::database::models::UserRole;
use crate::routes::auth::shared::parse_vk_id;
use crate::routes::schema::ResponseError;
use crate::routes::schema::user::UserResponse;
use actix_web::{post, web};
use web::Json;

async fn sign_up_combined(
    data: SignUpData,
    app_state: &web::Data<AppState>,
) -> Result<UserResponse, ErrorCode> {
    // If user selected forbidden role.
    if data.role == UserRole::Admin {
        return Err(ErrorCode::DisallowedRole);
    }

    if !app_state
        .get_schedule_snapshot("eng_polytechnic")
        .await
        .unwrap()
        .data
        .groups
        .contains_key(&data.group)
    {
        return Err(ErrorCode::InvalidGroupName);
    }

    // If user with specified username already exists.
    if driver::users::contains_by_username(&app_state, &data.username).await {
        return Err(ErrorCode::UsernameAlreadyExists);
    }

    // If user with specified VKID already exists.
    if let Some(id) = data.vk_id {
        if driver::users::contains_by_vk_id(&app_state, id).await {
            return Err(ErrorCode::VkAlreadyExists);
        }
    }

    let user = data.into();
    driver::users::insert(&app_state, &user).await.unwrap();

    Ok(UserResponse::from(&user)).into()
}

#[utoipa::path(responses(
    (status = OK, body = UserResponse),
    (status = NOT_ACCEPTABLE, body = ResponseError<ErrorCode>)
))]
#[post("/sign-up")]
pub async fn sign_up(data_json: Json<Request>, app_state: web::Data<AppState>) -> ServiceResponse {
    let data = data_json.into_inner();

    sign_up_combined(
        SignUpData {
            username: data.username,
            password: Some(data.password),
            vk_id: None,
            group: data.group,
            role: data.role,
            version: data.version,
        },
        &app_state,
    )
    .await
    .into()
}

#[utoipa::path(responses(
    (status = OK, body = UserResponse),
    (status = NOT_ACCEPTABLE, body = ResponseError<ErrorCode>)
))]
#[post("/sign-up-vk")]
pub async fn sign_up_vk(
    data_json: Json<vk::Request>,
    app_state: web::Data<AppState>,
) -> ServiceResponse {
    let data = data_json.into_inner();

    match parse_vk_id(&data.access_token, app_state.get_env().vk_id.client_id) {
        Ok(id) => {
            sign_up_combined(
                SignUpData {
                    username: data.username,
                    password: None,
                    vk_id: Some(id),
                    group: data.group,
                    role: data.role,
                    version: data.version,
                },
                &app_state,
            )
            .await
        }
        Err(_) => Err(ErrorCode::InvalidVkAccessToken),
    }
    .into()
}

mod schema {
    use crate::database::models::{User, UserRole};
    use crate::routes::schema::user::UserResponse;
    use crate::utility;
    use actix_macros::ErrResponse;
    use derive_more::Display;
    use objectid::ObjectId;
    use serde::{Deserialize, Serialize};

    /// WEB

    #[derive(Serialize, Deserialize, utoipa::ToSchema)]
    #[schema(as = SignUp::Request)]
    pub struct Request {
        /// User name.
        #[schema(examples("n08i40k"))]
        pub username: String,

        /// Password.
        pub password: String,

        /// Group.
        #[schema(examples("ИС-214/23"))]
        pub group: String,

        /// Role.
        pub role: UserRole,

        /// Version of the installed Polytechnic+ application.
        #[schema(examples("3.0.0"))]
        pub version: String,
    }

    pub mod vk {
        use crate::database::models::UserRole;
        use serde::{Deserialize, Serialize};

        #[derive(Serialize, Deserialize, utoipa::ToSchema)]
        #[serde(rename_all = "camelCase")]
        #[schema(as = SignUpVk::Request)]
        pub struct Request {
            /// VK ID token.
            pub access_token: String,

            /// User name.
            #[schema(examples("n08i40k"))]
            pub username: String,

            /// Group.
            #[schema(examples("ИС-214/23"))]
            pub group: String,

            /// Role.
            pub role: UserRole,

            /// Version of the installed Polytechnic+ application.
            #[schema(examples("3.0.0"))]
            pub version: String,
        }
    }

    pub type ServiceResponse = crate::routes::schema::Response<UserResponse, ErrorCode>;

    #[derive(Clone, Serialize, Display, utoipa::ToSchema, ErrResponse)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    #[schema(as = SignUp::ErrorCode)]
    #[status_code = "actix_web::http::StatusCode::NOT_ACCEPTABLE"]
    pub enum ErrorCode {
        /// Conveyed the role of Admin.
        #[display("Conveyed the role of Admin.")]
        DisallowedRole,

        /// Unknown name of the group.
        #[display("Unknown name of the group.")]
        InvalidGroupName,

        /// User with this name is already registered.
        #[display("User with this name is already registered.")]
        UsernameAlreadyExists,

        /// Invalid VK ID token.
        #[display("Invalid VK ID token.")]
        InvalidVkAccessToken,

        /// User with such an account VK is already registered.
        #[display("User with such an account VK is already registered.")]
        VkAlreadyExists,
    }

    /// Internal

    /// Data for registration.
    pub struct SignUpData {
        // TODO: сделать ограничение на минимальную и максимальную длину при регистрации и смене.
        /// User name.
        pub username: String,

        /// Password.
        ///
        /// Should be present even if registration occurs using the VK ID token.
        pub password: Option<String>,

        /// Account identifier VK.
        pub vk_id: Option<i32>,

        /// Group.
        pub group: String,

        /// Role.
        pub role: UserRole,

        /// Version of the installed Polytechnic+ application.
        pub version: String,
    }

    impl Into<User> for SignUpData {
        fn into(self) -> User {
            assert_ne!(self.password.is_some(), self.vk_id.is_some());

            let id = ObjectId::new().unwrap().to_string();
            let access_token = Some(utility::jwt::encode(&id));

            User {
                id,
                username: self.username,
                password: self
                    .password
                    .map(|x| bcrypt::hash(x, bcrypt::DEFAULT_COST).unwrap()),
                vk_id: self.vk_id,
                telegram_id: None,
                access_token,
                group: Some(self.group),
                role: self.role,
                android_version: Some(self.version),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::database::driver;
    use crate::database::models::UserRole;
    use crate::routes::auth::sign_up::schema::Request;
    use crate::routes::auth::sign_up::sign_up;
    use crate::test_env::tests::{static_app_state, test_app_state, test_env};
    use actix_test::test_app;
    use actix_web::dev::ServiceResponse;
    use actix_web::http::Method;
    use actix_web::http::StatusCode;
    use actix_web::test;

    struct SignUpPartial<'a> {
        username: &'a str,
        group: &'a str,
        role: UserRole,
    }

    async fn sign_up_client(data: SignUpPartial<'_>) -> ServiceResponse {
        let app = test_app(test_app_state().await, sign_up).await;

        let req = test::TestRequest::with_uri("/sign-up")
            .method(Method::POST)
            .set_json(Request {
                username: data.username.to_string(),
                password: "example".to_string(),
                group: data.group.to_string(),
                role: data.role.clone(),
                version: "1.0.0".to_string(),
            })
            .to_request();

        test::call_service(&app, req).await
    }

    #[actix_web::test]
    async fn sign_up_valid() {
        // prepare

        test_env();

        let app_state = static_app_state().await;
        driver::users::delete_by_username(&app_state, &"test::sign_up_valid".to_string()).await;

        // test

        let resp = sign_up_client(SignUpPartial {
            username: "test::sign_up_valid",
            group: "ИС-214/23",
            role: UserRole::Student,
        })
        .await;

        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[actix_web::test]
    async fn sign_up_multiple() {
        // prepare

        test_env();

        let app_state = static_app_state().await;
        driver::users::delete_by_username(&app_state, &"test::sign_up_multiple".to_string()).await;

        let create = sign_up_client(SignUpPartial {
            username: "test::sign_up_multiple",
            group: "ИС-214/23",
            role: UserRole::Student,
        })
        .await;

        assert_eq!(create.status(), StatusCode::OK);

        let resp = sign_up_client(SignUpPartial {
            username: "test::sign_up_multiple",
            group: "ИС-214/23",
            role: UserRole::Student,
        })
        .await;

        assert_eq!(resp.status(), StatusCode::NOT_ACCEPTABLE);
    }

    #[actix_web::test]
    async fn sign_up_invalid_role() {
        test_env();

        // test
        let resp = sign_up_client(SignUpPartial {
            username: "test::sign_up_invalid_role",
            group: "ИС-214/23",
            role: UserRole::Admin,
        })
        .await;

        assert_eq!(resp.status(), StatusCode::NOT_ACCEPTABLE);
    }

    #[actix_web::test]
    async fn sign_up_invalid_group() {
        test_env();

        // test
        let resp = sign_up_client(SignUpPartial {
            username: "test::sign_up_invalid_group",
            group: "invalid_group",
            role: UserRole::Student,
        })
        .await;

        assert_eq!(resp.status(), StatusCode::NOT_ACCEPTABLE);
    }
}
