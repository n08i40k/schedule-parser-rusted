use actix_web::body::EitherBody;
use actix_web::error::JsonPayloadError;
use actix_web::http::StatusCode;
use actix_web::{HttpRequest, HttpResponse, Responder};
use serde::Serialize;

pub struct IResponse<T: Serialize, E: Serialize>(pub Result<T, E>);

pub trait ErrorToHttpCode {
    fn to_http_status_code(&self) -> StatusCode;
}

impl<T: Serialize, E: Serialize + ErrorToHttpCode> Responder for IResponse<T, E> {
    type Body = EitherBody<String>;

    fn respond_to(self, _: &HttpRequest) -> HttpResponse<Self::Body> {
        match serde_json::to_string(&self.0) {
            Ok(body) => {
                let code = match &self.0 {
                    Ok(_) => StatusCode::OK,
                    Err(e) => e.to_http_status_code(),
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

pub mod user {
    use crate::database::models::{User, UserRole};
    use serde::Serialize;

    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct ResponseOk {
        id: String,
        username: String,
        group: String,
        role: UserRole,
        vk_id: Option<i32>,
        access_token: String,
    }

    impl ResponseOk {
        pub fn from_user(user: &User) -> Self {
            ResponseOk {
                id: user.id.clone(),
                username: user.username.clone(),
                group: user.group.clone(),
                role: user.role.clone(),
                vk_id: user.vk_id.clone(),
                access_token: user.access_token.clone(),
            }
        }
    }
}
