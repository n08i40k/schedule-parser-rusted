use self::schema::*;
use crate::routes::schema::ResponseError;
use crate::state::AppState;
use actix_web::{post, web};
use serde::Deserialize;
use std::collections::HashMap;
use uuid::Uuid;

#[allow(dead_code)]
#[derive(Deserialize)]
struct VkIdAuthResponse {
    refresh_token: String,
    access_token: String,
    id_token: String,
    token_type: String,
    expires_in: i32,
    user_id: i32,
    state: String,
    scope: String,
}

#[utoipa::path(responses(
    (status = OK, body = Response),
    (
        status = NOT_ACCEPTABLE,
        body = ResponseError<ErrorCode>,
        example = json!({
            "code": "VK_ID_ERROR",
            "message": "VK server returned an error"
        })
    ),
))]
#[post("/oauth")]
async fn oauth(data: web::Json<Request>, app_state: web::Data<AppState>) -> ServiceResponse {
    let data = data.into_inner();
    let state = Uuid::new_v4().simple().to_string();

    let vk_id = &app_state.get_env().vk_id;
    let client_id = vk_id.client_id.clone().to_string();

    let mut params = HashMap::new();
    params.insert("grant_type", "authorization_code");
    params.insert("client_id", client_id.as_str());
    params.insert("state", state.as_str());
    params.insert("code_verifier", data.code_verifier.as_str());
    params.insert("code", data.code.as_str());
    params.insert("device_id", data.device_id.as_str());
    params.insert("redirect_uri", vk_id.redirect_url.as_str());

    let client = reqwest::Client::new();
    match client
        .post("https://id.vk.com/oauth2/auth")
        .form(&params)
        .send()
        .await
    {
        Ok(res) => {
            if !res.status().is_success() {
                return Err(ErrorCode::VkIdError).into();
            }

            match res.json::<VkIdAuthResponse>().await {
                Ok(auth_data) => Ok(Response {
                    access_token: auth_data.id_token,
                }),
                Err(error) => {
                    sentry::capture_error(&error);

                    Err(ErrorCode::VkIdError)
                }
            }
        }
        Err(_) => Err(ErrorCode::VkIdError),
    }
    .into()
}

mod schema {
    use actix_macros::{ErrResponse, OkResponse};
    use derive_more::Display;
    use serde::{Deserialize, Serialize};
    use utoipa::ToSchema;

    pub type ServiceResponse = crate::routes::schema::Response<Response, ErrorCode>;

    #[derive(Deserialize, ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[schema(as = VkIdOAuth::Request)]
    pub struct Request {
        /// Код подтверждения authorization_code.
        pub code: String,

        /// Parameter to protect transmitted data.
        pub code_verifier: String,

        /// Device ID.
        pub device_id: String,
    }

    #[derive(Serialize, ToSchema, OkResponse)]
    #[serde(rename_all = "camelCase")]
    #[schema(as = VkIdOAuth::Response)]
    pub struct Response {
        /// ID token.
        pub access_token: String,
    }

    #[derive(Clone, Serialize, Display, ToSchema, ErrResponse)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    #[schema(as = VkIdOAuth::ErrorCode)]
    #[status_code = "actix_web::http::StatusCode::NOT_ACCEPTABLE"]
    pub enum ErrorCode {
        /// VK server returned an error.
        #[display("VK server returned an error")]
        VkIdError,
    }
}
