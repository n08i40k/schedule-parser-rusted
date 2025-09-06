use actix_web::body::EitherBody;
use actix_web::error::JsonPayloadError;
use actix_web::http::StatusCode;
use actix_web::{HttpRequest, HttpResponse, Responder};
use serde::{Serialize, Serializer};
use std::convert::Into;
use std::fmt::Display;
use utoipa::PartialSchema;

pub struct Response<T, E>(pub Result<T, E>)
where
    T: Serialize + PartialSchema + PartialOkResponse,
    E: Serialize + PartialSchema + Display + PartialErrResponse;

/// Transform Response<T, E> into Result<T, E>
impl<T, E> From<Response<T, E>> for Result<T, E>
where
    T: Serialize + PartialSchema + PartialOkResponse,
    E: Serialize + PartialSchema + Display + PartialErrResponse,
{
    fn from(value: Response<T, E>) -> Self {
        value.0
    }
}

/// Transform T into Response<T, E>
impl<T, E> From<Result<T, E>> for Response<T, E>
where
    T: Serialize + PartialSchema + PartialOkResponse,
    E: Serialize + PartialSchema + Display + PartialErrResponse,
{
    fn from(value: Result<T, E>) -> Self {
        Response(value)
    }
}

/// Serialize Response<T, E>
impl<T, E> Serialize for Response<T, E>
where
    T: Serialize + PartialSchema + PartialOkResponse,
    E: Serialize + PartialSchema + Display + PartialErrResponse + Clone + Into<ResponseError<E>>,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match &self.0 {
            Ok(ok) => serializer.serialize_some(&ok),
            Err(err) => serializer.serialize_some(&err.clone().into()),
        }
    }
}

/// Transform Response<T, E> to HttpResponse<String>
impl<T, E> Responder for Response<T, E>
where
    T: Serialize + PartialSchema + PartialOkResponse,
    E: Serialize + PartialSchema + Display + PartialErrResponse + Clone + Into<ResponseError<E>>,
{
    type Body = EitherBody<String>;

    fn respond_to(mut self, request: &HttpRequest) -> HttpResponse<Self::Body> {
        match serde_json::to_string(&self) {
            Ok(body) => {
                let code = match &self.0 {
                    Ok(_) => StatusCode::OK,
                    Err(e) => e.status_code(),
                };

                let mut response = match HttpResponse::build(code)
                    .content_type(mime::APPLICATION_JSON)
                    .message_body(body)
                {
                    Ok(res) => res.map_into_left_body(),
                    Err(err) => HttpResponse::from_error(err).map_into_right_body(),
                };

                if let Ok(ok) = &mut self.0 {
                    ok.post_process(request, &mut response);
                }

                response
            }

            Err(err) => {
                HttpResponse::from_error(JsonPayloadError::Serialize(err)).map_into_right_body()
            }
        }
    }
}

/// Трейт для всех положительных ответов от сервера
pub trait PartialOkResponse {
    fn post_process(
        &mut self,
        _request: &HttpRequest,
        _response: &mut HttpResponse<EitherBody<String>>,
    ) {
    }
}

impl PartialOkResponse for () {}

/// Трейт для всех отрицательных ответов от сервера
pub trait PartialErrResponse {
    fn status_code(&self) -> StatusCode;
}

/// ResponseError<T>
#[derive(Serialize, utoipa::ToSchema)]
pub struct ResponseError<T: Serialize + PartialSchema + Clone> {
    pub code: T,
    pub message: String,
}

impl<T> From<T> for ResponseError<T>
where
    T: Serialize + PartialSchema + Display + Clone,
{
    fn from(code: T) -> Self {
        Self {
            message: format!("{}", code),
            code,
        }
    }
}

pub mod user {
    use actix_macros::{OkResponse, ResponderJson};
    use database::entity::sea_orm_active_enums::UserRole;
    use database::entity::User;
    use serde::Serialize;

    //noinspection SpellCheckingInspection
    /// Используется для скрытия чувствительных полей, таких как хеш пароля
    #[derive(Serialize, utoipa::ToSchema, ResponderJson, OkResponse)]
    #[serde(rename_all = "camelCase")]
    pub struct UserResponse {
        /// UUID
        #[schema(examples("67dcc9a9507b0000772744a2"))]
        pub id: String,

        /// Имя пользователя
        #[schema(examples("n08i40k"))]
        pub username: String,

        /// Группа
        #[schema(examples("ИС-214/23"))]
        pub group: Option<String>,

        /// Роль
        pub role: UserRole,

        /// Идентификатор привязанного аккаунта VK
        #[schema(examples(498094647, json!(null)))]
        pub vk_id: Option<i32>,

        /// Идентификатор привязанного аккаунта Telegram
        #[schema(examples(996004735, json!(null)))]
        pub telegram_id: Option<i64>,

        /// JWT токен доступа
        #[schema(examples(
            "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJpZCI6IjY3ZGNjOWE5NTA3YjAwMDA3NzI3NDRhMiIsImlhdCI6IjE3NDMxMDgwOTkiLCJleHAiOiIxODY5MjUyMDk5In0.rMgXRb3JbT9AvLK4eiY9HMB5LxgUudkpQyoWKOypZFY"
        ))]
        pub access_token: Option<String>,
    }

    impl UserResponse {
        pub fn from_user_with_token(user: User, access_token: String) -> Self {
            Self {
                id: user.id.clone(),
                username: user.username.clone(),
                group: user.group.clone(),
                role: user.role.clone(),
                vk_id: user.vk_id,
                telegram_id: user.telegram_id,
                access_token: Some(access_token),
            }
        }
    }

    /// Create UserResponse from User ref.
    impl From<&User> for UserResponse {
        fn from(user: &User) -> Self {
            Self {
                id: user.id.clone(),
                username: user.username.clone(),
                group: user.group.clone(),
                role: user.role.clone(),
                vk_id: user.vk_id,
                telegram_id: user.telegram_id,
                access_token: None,
            }
        }
    }

    /// Transform User to UserResponse.
    impl From<User> for UserResponse {
        fn from(user: User) -> Self {
            Self {
                id: user.id,
                username: user.username,
                group: user.group,
                role: user.role,
                vk_id: user.vk_id,
                telegram_id: user.telegram_id,
                access_token: None,
            }
        }
    }
}
