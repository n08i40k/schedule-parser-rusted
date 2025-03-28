use self::schema::*;
use crate::AppState;
use crate::database::driver;
use crate::database::models::UserRole;
use crate::routes::auth::shared::{Error, parse_vk_id};
use crate::routes::schema::user::UserResponse;
use crate::routes::schema::{IntoResponseAsError, ResponseError};
use actix_web::{post, web};
use rand::{Rng, rng};
use web::Json;

async fn sign_up(
    data: SignUpData,
    app_state: &web::Data<AppState>,
) -> Result<UserResponse, ErrorCode> {
    // If user selected forbidden role.
    if data.role == UserRole::Admin {
        return Err(ErrorCode::DisallowedRole);
    }

    // If specified group doesn't exist in schedule.
    let schedule_opt = app_state.schedule.lock().unwrap();

    if let Some(schedule) = &*schedule_opt {
        if !schedule.data.groups.contains_key(&data.group) {
            return Err(ErrorCode::InvalidGroupName);
        }
    }

    // If user with specified username already exists.
    if driver::users::contains_by_username(&app_state.database, &data.username) {
        return Err(ErrorCode::UsernameAlreadyExists);
    }

    // If user with specified VKID already exists.
    if let Some(id) = data.vk_id {
        if driver::users::contains_by_vk_id(&app_state.database, id) {
            return Err(ErrorCode::VkAlreadyExists);
        }
    }

    let user = data.into();
    driver::users::insert(&app_state.database, &user).unwrap();

    Ok(UserResponse::from(&user)).into()
}

#[utoipa::path(responses(
    (status = OK, body = UserResponse),
    (status = NOT_ACCEPTABLE, body = ResponseError<ErrorCode>)
))]
#[post("/sign-up")]
pub async fn sign_up_default(
    data_json: Json<Request>,
    app_state: web::Data<AppState>,
) -> ServiceResponse {
    let data = data_json.into_inner();

    sign_up(
        SignUpData {
            username: data.username,
            password: data.password,
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

    match parse_vk_id(&data.access_token) {
        Ok(id) => sign_up(
            SignUpData {
                username: data.username,
                password: rng()
                    .sample_iter(&rand::distr::Alphanumeric)
                    .take(16)
                    .map(char::from)
                    .collect(),
                vk_id: Some(id),
                group: data.group,
                role: data.role,
                version: data.version,
            },
            &app_state,
        )
        .await
        .into(),
        Err(err) => {
            if err != Error::Expired {
                eprintln!("Failed to parse vk id token!");
                eprintln!("{:?}", err);
            }

            ErrorCode::InvalidVkAccessToken.into_response()
        }
    }
}

mod schema {
    use crate::database::models::{User, UserRole};
    use crate::routes::schema::user::UserResponse;
    use crate::utility;
    use actix_macros::{IntoResponseError, StatusCode};
    use objectid::ObjectId;
    use serde::{Deserialize, Serialize};

    /// WEB

    #[derive(Serialize, Deserialize, utoipa::ToSchema)]
    #[schema(as = SignUp::Request)]
    pub struct Request {
        /// Имя пользователя
        #[schema(examples("n08i40k"))]
        pub username: String,

        /// Пароль
        pub password: String,

        /// Группа
        #[schema(examples("ИС-214/23"))]
        pub group: String,

        /// Роль
        pub role: UserRole,

        /// Версия установленного приложения Polytechnic+
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
            /// Токен VK ID
            pub access_token: String,

            /// Имя пользователя
            #[schema(examples("n08i40k"))]
            pub username: String,

            /// Группа
            #[schema(examples("ИС-214/23"))]
            pub group: String,

            /// Роль
            pub role: UserRole,

            /// Версия установленного приложения Polytechnic+
            #[schema(examples("3.0.0"))]
            pub version: String,
        }
    }

    pub type ServiceResponse = crate::routes::schema::Response<UserResponse, ErrorCode>;

    #[derive(Clone, Serialize, utoipa::ToSchema, IntoResponseError, StatusCode)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    #[schema(as = SignUp::ErrorCode)]
    #[status_code = "actix_web::http::StatusCode::NOT_ACCEPTABLE"]
    pub enum ErrorCode {
        /// Передана роль ADMIN
        DisallowedRole,

        /// Неизвестное название группы
        InvalidGroupName,

        /// Пользователь с таким именем уже зарегистрирован
        UsernameAlreadyExists,

        /// Недействительный токен VK ID
        InvalidVkAccessToken,

        /// Пользователь с таким аккаунтом VK уже зарегистрирован
        VkAlreadyExists,
    }

    /// Internal

    /// Данные для регистрации
    pub struct SignUpData {
        /// Имя пользователя
        pub username: String,

        /// Пароль
        ///
        /// Должен присутствовать даже если регистрация происходит с помощью токена VK ID
        pub password: String,

        /// Идентификатор аккаунта VK
        pub vk_id: Option<i32>,

        /// Группа
        pub group: String,

        /// Роль
        pub role: UserRole,

        /// Версия установленного приложения Polytechnic+
        pub version: String,
    }

    impl Into<User> for SignUpData {
        fn into(self) -> User {
            let id = ObjectId::new().unwrap().to_string();
            let access_token = utility::jwt::encode(&id);

            User {
                id,
                username: self.username,
                password: bcrypt::hash(self.password, bcrypt::DEFAULT_COST).unwrap(),
                vk_id: self.vk_id,
                access_token,
                group: self.group,
                role: self.role,
                version: self.version,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::database::driver;
    use crate::database::models::UserRole;
    use crate::routes::auth::sign_up::schema::Request;
    use crate::routes::auth::sign_up::sign_up_default;
    use crate::test_env::tests::{static_app_state, test_app_state, test_env};
    use actix_test::test_app;
    use actix_web::dev::ServiceResponse;
    use actix_web::http::Method;
    use actix_web::http::StatusCode;
    use actix_web::test;

    struct SignUpPartial {
        username: String,
        group: String,
        role: UserRole,
    }

    async fn sign_up_client(data: SignUpPartial) -> ServiceResponse {
        let app = test_app(test_app_state(), sign_up_default).await;

        let req = test::TestRequest::with_uri("/sign-up")
            .method(Method::POST)
            .set_json(Request {
                username: data.username.clone(),
                password: "example".to_string(),
                group: data.group.clone(),
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

        let app_state = static_app_state();
        driver::users::delete_by_username(&app_state.database, &"test::sign_up_valid".to_string());

        // test

        let resp = sign_up_client(SignUpPartial {
            username: "test::sign_up_valid".to_string(),
            group: "ИС-214/23".to_string(),
            role: UserRole::Student,
        })
        .await;

        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[actix_web::test]
    async fn sign_up_multiple() {
        // prepare

        test_env();

        let app_state = static_app_state();
        driver::users::delete_by_username(
            &app_state.database,
            &"test::sign_up_multiple".to_string(),
        );

        let create = sign_up_client(SignUpPartial {
            username: "test::sign_up_multiple".to_string(),
            group: "ИС-214/23".to_string(),
            role: UserRole::Student,
        })
        .await;

        assert_eq!(create.status(), StatusCode::OK);

        let resp = sign_up_client(SignUpPartial {
            username: "test::sign_up_multiple".to_string(),
            group: "ИС-214/23".to_string(),
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
            username: "test::sign_up_invalid_role".to_string(),
            group: "ИС-214/23".to_string(),
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
            username: "test::sign_up_invalid_group".to_string(),
            group: "invalid_group".to_string(),
            role: UserRole::Student,
        })
        .await;

        assert_eq!(resp.status(), StatusCode::NOT_ACCEPTABLE);
    }
}
