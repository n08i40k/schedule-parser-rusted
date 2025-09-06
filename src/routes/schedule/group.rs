use self::schema::*;
use crate::AppState;
use crate::extractors::base::AsyncExtractor;
use crate::routes::schedule::schema::ScheduleEntryResponse;
use crate::routes::schema::ResponseError;
use actix_web::{get, web};
use database::entity::User;

#[utoipa::path(responses(
    (status = OK, body = ScheduleEntryResponse),
    (
        status = SERVICE_UNAVAILABLE,
        body = ResponseError<ErrorCode>,
        example = json!({
            "code": "NO_SCHEDULE",
            "message": "Schedule not parsed yet."
        })
    ),
    (
        status = NOT_FOUND,
        body = ResponseError<ErrorCode>,
        example = json!({
            "code": "NOT_FOUND",
            "message": "Required group not found."
        })
    ),
))]
#[get("/group")]
pub async fn group(user: AsyncExtractor<User>, app_state: web::Data<AppState>) -> ServiceResponse {
    match &user.into_inner().group {
        None => Err(ErrorCode::SignUpNotCompleted),

        Some(group) => match app_state
            .get_schedule_snapshot("eng_polytechnic")
            .await
            .unwrap()
            .data
            .groups
            .get(group)
        {
            None => Err(ErrorCode::NotFound),

            Some(entry) => Ok(entry.clone().into()),
        },
    }
    .into()
}

mod schema {
    use crate::routes::schedule::schema::ScheduleEntryResponse;
    use actix_macros::ErrResponse;
    use derive_more::Display;
    use serde::Serialize;
    use utoipa::ToSchema;

    pub type ServiceResponse = crate::routes::schema::Response<ScheduleEntryResponse, ErrorCode>;

    #[derive(Clone, Serialize, Display, ToSchema, ErrResponse)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    #[schema(as = GroupSchedule::ErrorCode)]
    pub enum ErrorCode {
        /// The user tried to access the API without completing singing up.
        #[status_code = "actix_web::http::StatusCode::FORBIDDEN"]
        #[display("You have not completed signing up.")]
        SignUpNotCompleted,

        /// Group not found.
        #[status_code = "actix_web::http::StatusCode::NOT_FOUND"]
        #[display("Required group not found.")]
        NotFound,
    }
}
