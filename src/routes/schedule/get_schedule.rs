use self::schema::*;
use crate::app_state::AppState;
use crate::routes::schedule::schema::{ErrorCode, ScheduleView};
use crate::routes::schema::{IntoResponseAsError, ResponseError};
use actix_web::{get, web};

#[utoipa::path(responses(
    (status = OK, body = ScheduleView),
    (status = SERVICE_UNAVAILABLE, body = ResponseError<ErrorCode>)
))]
#[get("/")]
pub async fn get_schedule(app_state: web::Data<AppState>) -> ServiceResponse {
    match ScheduleView::try_from(&app_state) {
        Ok(res) => Ok(res).into(),
        Err(e) => match e {
            ErrorCode::NoSchedule => ErrorCode::NoSchedule.into_response(),
        },
    }
}

mod schema {
    use crate::routes::schedule::schema::{ErrorCode, ScheduleView};

    pub type ServiceResponse = crate::routes::schema::Response<ScheduleView, ErrorCode>;
}
