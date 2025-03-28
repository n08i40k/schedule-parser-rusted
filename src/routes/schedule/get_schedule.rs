use self::schema::*;
use crate::app_state::AppState;
use crate::routes::schedule::schema::{Error, ScheduleView};
use crate::routes::schema::{IntoResponseAsError, ResponseError};
use actix_web::{get, web};

#[utoipa::path(responses(
    (status = OK, body = ScheduleView),
    (status = SERVICE_UNAVAILABLE, body = ResponseError<ErrorCode>)
))]
#[get("/")]
pub async fn get_schedule(app_state: web::Data<AppState>) -> Response {
    match ScheduleView::try_from(app_state.get_ref()) {
        Ok(res) => Ok(res).into(),
        Err(e) => match e {
            Error::NoSchedule => ErrorCode::NoSchedule.into_response(),
        },
    }
}

mod schema {
    use crate::routes::schedule::schema::ScheduleView;
    use actix_macros::{IntoResponseErrorNamed, StatusCode};
    use derive_more::Display;
    use serde::Serialize;
    use utoipa::ToSchema;

    pub type Response = crate::routes::schema::Response<ScheduleView, ErrorCode>;

    #[derive(Clone, Serialize, ToSchema, StatusCode, Display, IntoResponseErrorNamed)]
    #[status_code = "actix_web::http::StatusCode::SERVICE_UNAVAILABLE"]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    #[schema(as = ScheduleView::ErrorCode)]
    pub enum ErrorCode {
        #[display("Schedule not parsed yet")]
        NoSchedule,
    }
}
