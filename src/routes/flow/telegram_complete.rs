use self::schema::*;
use crate::extractors::base::AsyncExtractor;
use crate::routes::schema::ResponseError;
use crate::AppState;
use actix_web::{post, web};
use database::entity::User;
use database::query::Query;
use database::sea_orm::{ActiveModelTrait, IntoActiveModel, Set};
use std::ops::Deref;
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
    let user = user.into_inner();

    // проверка на перезапись уже имеющихся данных
    if user.group.is_some() {
        return Err(ErrorCode::AlreadyCompleted).into();
    }

    let data = data.into_inner();

    let db = app_state.get_database();
    let mut active_user = user.clone().into_active_model();

    // замена существующего имени, если оно отличается
    if user.username != data.username {
        if Query::is_user_exists_by_username(db, &data.username)
            .await
            .unwrap()
        {
            return Err(ErrorCode::UsernameAlreadyExists).into();
        }

        active_user.username = Set(data.username);
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

    active_user.group = Set(Some(data.group));

    active_user
        .update(db)
        .await
        .expect("Failed to update user");

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
        #[display("This flow is already completed.")]
        #[status_code = "actix_web::http::StatusCode::CONFLICT"]
        AlreadyCompleted,

        #[display("User with that name already exists.")]
        #[status_code = "actix_web::http::StatusCode::BAD_REQUEST"]
        UsernameAlreadyExists,

        #[display("The required group does not exist.")]
        #[status_code = "actix_web::http::StatusCode::BAD_REQUEST"]
        InvalidGroupName,
    }
}
