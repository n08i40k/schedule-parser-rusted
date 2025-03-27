use actix_web::body::EitherBody;
use actix_web::error::JsonPayloadError;
use actix_web::http::StatusCode;
use actix_web::{HttpRequest, HttpResponse, Responder};
use serde::{Serialize, Serializer};
use utoipa::PartialSchema;

pub struct IResponse<T, E>(pub Result<T, E>)
where
    T: Serialize + PartialSchema,
    E: Serialize + PartialSchema + Clone + HttpStatusCode;

impl<T, E> Into<Result<T, E>> for IResponse<T, E>
where
    T: Serialize + PartialSchema,
    E: Serialize + PartialSchema + Clone + HttpStatusCode,
{
    fn into(self) -> Result<T, E> {
        self.0
    }
}

impl<T, E> From<E> for IResponse<T, E>
where
    T: Serialize + PartialSchema,
    E: Serialize + PartialSchema + Clone + HttpStatusCode,
{
    fn from(value: E) -> Self {
        IResponse(Err(value))
    }
}

pub trait HttpStatusCode {
    fn status_code(&self) -> StatusCode;
}

impl<T, E> IResponse<T, E>
where
    T: Serialize + PartialSchema,
    E: Serialize + PartialSchema + Clone + HttpStatusCode,
{
    pub fn new(result: Result<T, E>) -> Self {
        IResponse(result)
    }
}

impl<T, E> Serialize for IResponse<T, E>
where
    T: Serialize + PartialSchema,
    E: Serialize + PartialSchema + Clone + HttpStatusCode,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match &self.0 {
            Ok(ok) => serializer.serialize_some::<T>(&ok),
            Err(err) => serializer.serialize_some::<ResponseError<E>>(&ResponseError::new(err)),
        }
    }
}

impl<T, E> Responder for IResponse<T, E>
where
    T: Serialize + PartialSchema,
    E: Serialize + PartialSchema + Clone + HttpStatusCode,
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

#[derive(Serialize, utoipa::ToSchema)]
pub struct ResponseError<T: Serialize + PartialSchema> {
    code: T,
}

impl<T: Serialize + PartialSchema + Clone> ResponseError<T> {
    fn new(status_code: &T) -> Self {
        ResponseError {
            code: status_code.clone(),
        }
    }
}

pub mod user {
    use crate::database::models::{User, UserRole};
    use actix_macros::{IntoIResponse, ResponderJson};
    use serde::Serialize;

    #[derive(Serialize, utoipa::ToSchema, IntoIResponse, ResponderJson)]
    #[serde(rename_all = "camelCase")]
    pub struct UserResponse {
        #[schema(examples("67dcc9a9507b0000772744a2"))]
        id: String,

        #[schema(examples("n08i40k"))]
        username: String,

        #[schema(examples("ИС-214/23"))]
        group: String,

        role: UserRole,

        #[schema(examples(498094647, json!(null)))]
        vk_id: Option<i32>,

        #[schema(examples(
            "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzI1NiJ9.eyJpZCI6IjY3ZGNjOWE5NTA3YjAwMDA3NzI3NDRhMiIsImlhdCI6IjE3NDMxMDgwOTkiLCJleHAiOiIxODY5MjUyMDk5In0.rMgXRb3JbT9AvLK4eiY9HMB5LxgUudkpQyoWKOypZFY"
        ))]
        access_token: String,
    }

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
