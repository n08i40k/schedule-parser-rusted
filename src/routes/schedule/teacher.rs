use self::schema::*;
use crate::AppState;
use crate::routes::schema::ResponseError;
use actix_web::{get, web};
use providers::base::ScheduleEntry;

#[utoipa::path(responses(
    (status = OK, body = ScheduleEntry),
    (
        status = NOT_FOUND,
        body = ResponseError<ErrorCode>,
        example = json!({
            "code": "NOT_FOUND",
            "message": "Required teacher not found."
        })
    ),
))]
#[get("/teacher/{name}")]
pub async fn teacher(name: web::Path<String>, app_state: web::Data<AppState>) -> ServiceResponse {
    match app_state
        .get_schedule_snapshot("eng_polytechnic")
        .await
        .unwrap()
        .data
        .teachers
        .get(&name.into_inner())
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
    #[schema(as = TeacherSchedule::ErrorCode)]
    pub enum ErrorCode {
        /// Teacher not found.
        #[status_code = "actix_web::http::StatusCode::NOT_FOUND"]
        #[display("Required teacher not found.")]
        NotFound,
    }
}
