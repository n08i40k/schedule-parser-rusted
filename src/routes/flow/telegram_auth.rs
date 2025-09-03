use self::schema::*;
use crate::database::driver;
use crate::database::driver::users::UserSave;
use crate::database::models::{User, UserRole};
use crate::routes::schema::ResponseError;
use crate::utility::telegram::{WebAppInitDataMap, WebAppUser};
use crate::{AppState, utility};
use actix_web::{post, web};
use chrono::{DateTime, Duration, Utc};
use objectid::ObjectId;
use std::sync::Arc;
use web::Json;

#[utoipa::path(responses(
    (status = OK, body = Response),
    (status = UNAUTHORIZED, body = ResponseError<ErrorCode>),
))]
#[post("/telegram-auth")]
pub async fn telegram_auth(
    data_json: Json<Request>,
    app_state: web::Data<AppState>,
) -> ServiceResponse {
    let init_data = WebAppInitDataMap::from_str(data_json.into_inner().init_data);

    {
        let env = &app_state.get_env().telegram;

        if let Err(error) = init_data.verify(env.bot_id, env.test_dc) {
            return Err(ErrorCode::InvalidInitData(Arc::new(error))).into();
        }
    }

    let auth_date = DateTime::<Utc>::from_timestamp(
        init_data
            .data_map
            .get("auth_date")
            .unwrap()
            .parse()
            .unwrap(),
        0,
    )
    .unwrap();

    if Utc::now() - auth_date > Duration::minutes(5) {
        return Err(ErrorCode::ExpiredInitData).into();
    }

    let web_app_user =
        serde_json::from_str::<WebAppUser>(init_data.data_map.get("user").unwrap()).unwrap();

    let mut user = {
        match driver::users::get_by_telegram_id(&app_state, web_app_user.id).await {
            Ok(value) => Ok(value),
            Err(_) => {
                let new_user = User {
                    id: ObjectId::new().unwrap().to_string(),
                    username: format!("telegram_{}", web_app_user.id), // можно оставить, а можно поменять
                    password: None,                                    // ибо нехуй
                    vk_id: None,
                    telegram_id: Some(web_app_user.id),
                    access_token: None, // установится ниже
                    group: None,
                    role: UserRole::Student, // TODO: при реге проверять данные
                    android_version: None,
                };

                driver::users::insert(&app_state, &new_user)
                    .await
                    .map(|_| new_user)
            }
        }
        .expect("Failed to get or add user")
    };

    user.access_token = Some(utility::jwt::encode(&user.id));

    user.save(&app_state).await.expect("Failed to update user");

    Ok(Response::new(
        &*user.access_token.unwrap(),
        user.group.is_some(),
    ))
    .into()
}

mod schema {
    use crate::routes::schema::PartialOkResponse;
    use crate::state::AppState;
    use crate::utility::telegram::VerifyError;
    use actix_macros::ErrResponse;
    use actix_web::body::EitherBody;
    use actix_web::cookie::CookieBuilder;
    use actix_web::cookie::time::OffsetDateTime;
    use actix_web::{HttpRequest, HttpResponse, web};
    use derive_more::Display;
    use serde::{Deserialize, Serialize, Serializer};
    use std::ops::Add;
    use std::sync::Arc;
    use utoipa::ToSchema;

    #[derive(Debug, Deserialize, Serialize, ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[schema(as = Flow::TelegramAuth::Request)]
    pub struct Request {
        /// Telegram WebApp init data.
        pub init_data: String,
    }

    #[derive(Serialize, ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[schema(as = Flow::TelegramAuth::Response)]
    pub struct Response {
        // #[serde(skip)]       // TODO: я пока не придумал как не отдавать сырой токен в ответе
        // #[schema(ignore)]
        access_token: String,

        pub completed: bool,
    }

    impl Response {
        pub fn new(access_token: &str, completed: bool) -> Self {
            Self {
                access_token: access_token.to_string(),
                completed,
            }
        }
    }

    impl PartialOkResponse for Response {
        fn post_process(
            &mut self,
            request: &HttpRequest,
            response: &mut HttpResponse<EitherBody<String>>,
        ) -> () {
            let access_token = &self.access_token;

            let app_state = request.app_data::<web::Data<AppState>>().unwrap();
            let mini_app_host = &*app_state.get_env().telegram.mini_app_host;

            let cookie = CookieBuilder::new("access_token", access_token)
                .domain(mini_app_host)
                .path("/")
                .expires(
                    OffsetDateTime::now_utc().add(std::time::Duration::from_secs(60 * 60 * 24 * 7)),
                )
                .http_only(true)
                .secure(true)
                .finish();

            response.add_cookie(&cookie).unwrap();
        }
    }

    pub type ServiceResponse = crate::routes::schema::Response<Response, ErrorCode>;

    #[derive(Clone, ToSchema, Display, ErrResponse)]
    #[status_code = "actix_web::http::StatusCode::UNAUTHORIZED"]
    #[schema(as = Flow::TelegramAuth::ErrorCode)]
    pub enum ErrorCode {
        #[display("Invalid init data provided: {_0}")]
        #[schema(value_type = String)]
        InvalidInitData(Arc<VerifyError>),

        #[display("Expired init data provided.")]
        ExpiredInitData,
    }

    impl Serialize for ErrorCode {
        fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
        where
            S: Serializer,
        {
            match self {
                ErrorCode::InvalidInitData(_) => serializer.serialize_str("INVALID_INIT_DATA"),
                ErrorCode::ExpiredInitData => serializer.serialize_str("EXPIRED_INIT_DATA"),
            }
        }
    }
}