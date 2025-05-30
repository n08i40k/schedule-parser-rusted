use actix_web::body::EitherBody;
use actix_web::error::JsonPayloadError;
use actix_web::http::StatusCode;
use actix_web::{HttpRequest, HttpResponse, Responder};
use serde::{Serialize, Serializer};
use std::convert::Into;
use utoipa::PartialSchema;

pub struct Response<T, E>(pub Result<T, E>)
where
    T: Serialize + PartialSchema,
    E: Serialize + PartialSchema + Clone + PartialStatusCode;

pub trait PartialStatusCode {
    fn status_code(&self) -> StatusCode;
}

/// Transform Response<T, E> into Result<T, E>
impl<T, E> Into<Result<T, E>> for Response<T, E>
where
    T: Serialize + PartialSchema,
    E: Serialize + PartialSchema + Clone + PartialStatusCode,
{
    fn into(self) -> Result<T, E> {
        self.0
    }
}

/// Transform T into Response<T, E>
impl<T, E> From<Result<T, E>> for Response<T, E>
where
    T: Serialize + PartialSchema,
    E: Serialize + PartialSchema + Clone + PartialStatusCode,
{
    fn from(value: Result<T, E>) -> Self {
        Response(value)
    }
}

/// Serialize Response<T, E>
impl<T, E> Serialize for Response<T, E>
where
    T: Serialize + PartialSchema,
    E: Serialize + PartialSchema + Clone + PartialStatusCode + Into<ResponseError<E>>,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match &self.0 {
            Ok(ok) => serializer.serialize_some::<T>(&ok),
            Err(err) => serializer
                .serialize_some::<ResponseError<E>>(&ResponseError::<E>::from(err.clone().into())),
        }
    }
}

/// Transform Response<T, E> to HttpResponse<String>
impl<T, E> Responder for Response<T, E>
where
    T: Serialize + PartialSchema,
    E: Serialize + PartialSchema + Clone + PartialStatusCode + Into<ResponseError<E>>,
{
    type Body = EitherBody<String>;

    fn respond_to(self, _: &HttpRequest) -> HttpResponse<Self::Body> {
        match serde_json::to_string(&self) {
            Ok(body) => {
                let code = match &self.0 {
                    Ok(_) => StatusCode::OK,
                    Err(e) => e.status_code(),
                };

                match HttpResponse::build(code)
                    .content_type(mime::APPLICATION_JSON)
                    .message_body(body)
                {
                    Ok(res) => res.map_into_left_body(),
                    Err(err) => HttpResponse::from_error(err).map_into_right_body(),
                }
            }

            Err(err) => {
                HttpResponse::from_error(JsonPayloadError::Serialize(err)).map_into_right_body()
            }
        }
    }
}

/// ResponseError<T>
///
/// Field `message` is optional for backwards compatibility with Android App, that produces error if new fields will be added to JSON response.
#[derive(Serialize, utoipa::ToSchema)]
pub struct ResponseError<T: Serialize + PartialSchema> {
    pub code: T,

    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

pub trait IntoResponseAsError<T>
where
    T: Serialize + PartialSchema,
    Self: Serialize + PartialSchema + Clone + PartialStatusCode + Into<ResponseError<Self>>,
{
    fn into_response(self) -> Response<T, Self> {
        Response(Err(self))
    }
}

pub mod user {
    use crate::database::models::{User, UserRole};
    use actix_macros::ResponderJson;
    use serde::Serialize;

    //noinspection SpellCheckingInspection
    /// Используется для скрытия чувствительных полей, таких как хеш пароля или FCM
    #[derive(Serialize, utoipa::ToSchema, ResponderJson)]
    #[serde(rename_all = "camelCase")]
    pub struct UserResponse {
        /// UUID
        #[schema(examples("67dcc9a9507b0000772744a2"))]
        id: String,

        /// Имя пользователя
        #[schema(examples("n08i40k"))]
        username: String,

        /// Группа
        #[schema(examples("ИС-214/23"))]
        group: String,

        /// Роль
        role: UserRole,

        /// Идентификатор привязанного аккаунта VK
        #[schema(examples(498094647, json!(null)))]
        vk_id: Option<i32>,

        /// JWT токен доступа
        #[schema(examples(
            "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJpZCI6IjY3ZGNjOWE5NTA3YjAwMDA3NzI3NDRhMiIsImlhdCI6IjE3NDMxMDgwOTkiLCJleHAiOiIxODY5MjUyMDk5In0.rMgXRb3JbT9AvLK4eiY9HMB5LxgUudkpQyoWKOypZFY"
        ))]
        access_token: String,
    }

    /// Create UserResponse from User ref.
    impl From<&User> for UserResponse {
        fn from(user: &User) -> Self {
            UserResponse {
                id: user.id.clone(),
                username: user.username.clone(),
                group: user.group.clone(),
                role: user.role.clone(),
                vk_id: user.vk_id.clone(),
                access_token: user.access_token.clone(),
            }
        }
    }

    /// Transform User to UserResponse.
    impl From<User> for UserResponse {
        fn from(user: User) -> Self {
            UserResponse {
                id: user.id,
                username: user.username,
                group: user.group,
                role: user.role,
                vk_id: user.vk_id,
                access_token: user.access_token,
            }
        }
    }
}
