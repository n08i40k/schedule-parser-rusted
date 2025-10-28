use self::schema::*;
use crate::routes::schedule::schema::ScheduleEntryResponse;
use crate::routes::schema::ResponseError;
use crate::AppState;
use actix_web::{get, web};

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
#[get("/group/{group_name}")]
pub async fn group_by_name(
    path: web::Path<String>,
    app_state: web::Data<AppState>,
) -> ServiceResponse {
    let group_name = path.into_inner();

    match app_state
        .get_schedule_snapshot("eng_polytechnic")
        .await
        .unwrap()
        .data
        .groups
        .get(&group_name)
    {
        None => Err(ErrorCode::NotFound),
        Some(entry) => Ok(entry.clone().into()),
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
    #[schema(as = GroupByNameSchedule::ErrorCode)]
    pub enum ErrorCode {
        /// Group not found.
        #[status_code = "actix_web::http::StatusCode::NOT_FOUND"]
        #[display("Required group not found.")]
        NotFound,
    }
}
