use self::schema::*;
use crate::AppState;
use crate::database::driver;
use crate::database::driver::users::UserSave;
use crate::database::models::User;
use crate::extractors::base::AsyncExtractor;
use crate::routes::schema::ResponseError;
use actix_web::{post, web};
use web::Json;

#[utoipa::path(responses(
    (status = OK),
    (status = CONFLICT, body = ResponseError<ErrorCode>),
    (status = INTERNAL_SERVER_ERROR, body = ResponseError<ErrorCode>),
    (status = BAD_REQUEST, body = ResponseError<ErrorCode>)
))]
#[post("/telegram-complete")]
pub async fn telegram_complete(
    data: Json<Request>,
    app_state: web::Data<AppState>,
    user: AsyncExtractor<User>,
) -> ServiceResponse {
    let mut user = user.into_inner();

    // проверка на перезапись уже имеющихся данных
    if user.group.is_some() {
        return Err(ErrorCode::AlreadyCompleted).into();
    }

    let data = data.into_inner();

    // замена существующего имени, если оно отличается
    if user.username != data.username {
        if driver::users::contains_by_username(&app_state, &data.username).await {
            return Err(ErrorCode::UsernameAlreadyExists).into();
        }

        user.username = data.username;
    }

    // проверка на существование группы
    if !app_state
        .get_schedule_snapshot("eng_polytechnic")
        .await
        .unwrap()
        .data
        .groups
        .contains_key(&data.group)
    {
        return Err(ErrorCode::InvalidGroupName).into();
    }

    user.group = Some(data.group);

    user.save(&app_state).await.expect("Failed to update user");

    Ok(()).into()
}

mod schema {
    use actix_macros::ErrResponse;
    use derive_more::Display;
    use serde::{Deserialize, Serialize};
    use utoipa::ToSchema;

    #[derive(Debug, Deserialize, Serialize, ToSchema)]
    #[schema(as = Flow::TelegramFill::Request)]
    pub struct Request {
        /// Username.
        pub username: String,

        /// Group.
        pub group: String,
    }

    pub type ServiceResponse = crate::routes::schema::Response<(), ErrorCode>;

    #[derive(Clone, Serialize, Display, ToSchema, ErrResponse)]
    #[status_code = "actix_web::http::StatusCode::UNAUTHORIZED"]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    #[schema(as = Flow::TelegramFill::ErrorCode)]
    pub enum ErrorCode {
        #[display("This flow already completed.")]
        #[status_code = "actix_web::http::StatusCode::CONFLICT"]
        AlreadyCompleted,

        #[display("Username is already exists.")]
        #[status_code = "actix_web::http::StatusCode::BAD_REQUEST"]
        UsernameAlreadyExists,

        #[display("The required group does not exist.")]
        #[status_code = "actix_web::http::StatusCode::BAD_REQUEST"]
        InvalidGroupName,
    }
}
