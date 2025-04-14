use self::schema::*;
use crate::routes::schema::{IntoResponseAsError, ResponseError};
use crate::AppState;
use actix_web::{get, web};

#[utoipa::path(responses(
    (status = OK, body = Response),
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
            "message": "Required teacher not found."
        })
    ),
))]
#[get("/teacher/{name}")]
pub async fn teacher(
    name: web::Path<String>,
    app_state: web::Data<AppState>,
) -> ServiceResponse {
    // Prevent thread lock
    let schedule_lock = app_state.schedule.lock().unwrap();

    match schedule_lock.as_ref() {
        None => ErrorCode::NoSchedule.into_response(),
        Some(schedule) => match schedule.data.teachers.get(&name.into_inner()) {
            None => ErrorCode::NotFound.into_response(),
            Some(entry) => Ok(entry.clone().into()).into(),
        },
    }
}

mod schema {
    use crate::parser::schema::ScheduleEntry;
    use actix_macros::{IntoResponseErrorNamed, StatusCode};
    use chrono::{DateTime, NaiveDateTime, Utc};
    use derive_more::Display;
    use serde::Serialize;
    use utoipa::ToSchema;

    pub type ServiceResponse = crate::routes::schema::Response<Response, ErrorCode>;

    #[derive(Serialize, ToSchema)]
    #[schema(as = GetTeacher::Response)]
    #[serde(rename_all = "camelCase")]
    pub struct Response {
        /// Расписание преподавателя
        pub teacher: ScheduleEntry,

        /// Устаревшая переменная
        ///
        /// По умолчанию возвращается пустой список
        #[deprecated = "Will be removed in future versions"]
        pub updated: Vec<i32>,

        /// Устаревшая переменная
        ///
        /// По умолчанию начальная дата по Unix
        #[deprecated = "Will be removed in future versions"]
        pub updated_at: DateTime<Utc>,
    }

    #[allow(deprecated)]
    impl From<ScheduleEntry> for Response {
        fn from(teacher: ScheduleEntry) -> Self {
            Self {
                teacher,
                updated: Vec::new(),
                updated_at: NaiveDateTime::default().and_utc(),
            }
        }
    }

    #[derive(Clone, Serialize, ToSchema, StatusCode, Display, IntoResponseErrorNamed)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    #[schema(as = TeacherSchedule::ErrorCode)]
    pub enum ErrorCode {
        /// Расписания ещё не получены
        #[status_code = "actix_web::http::StatusCode::SERVICE_UNAVAILABLE"]
        #[display("Schedule not parsed yet.")]
        NoSchedule,

        /// Преподаватель не найден
        #[status_code = "actix_web::http::StatusCode::NOT_FOUND"]
        #[display("Required teacher not found.")]
        NotFound,
    }
}
