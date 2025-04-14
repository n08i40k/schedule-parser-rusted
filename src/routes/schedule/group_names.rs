use self::schema::*;
use crate::AppState;
use crate::routes::schedule::schema::ErrorCode;
use crate::routes::schema::{IntoResponseAsError, ResponseError};
use actix_web::{get, web};

#[utoipa::path(responses(
    (status = OK, body = Response),
    (status = SERVICE_UNAVAILABLE, body = ResponseError<ErrorCode>),
))]
#[get("/group-names")]
pub async fn group_names(app_state: web::Data<AppState>) -> ServiceResponse {
    // Prevent thread lock
    let schedule_lock = app_state.schedule.lock().unwrap();

    match schedule_lock.as_ref() {
        None => ErrorCode::NoSchedule.into_response(),
        Some(schedule) => {
            let mut names: Vec<String> = schedule.data.groups.keys().cloned().collect();
            names.sort();
            
            Ok(names.into()).into()
        }
    }
    .into()
}

mod schema {
    use crate::routes::schedule::schema::ErrorCode;
    use serde::Serialize;
    use utoipa::ToSchema;

    pub type ServiceResponse = crate::routes::schema::Response<Response, ErrorCode>;

    #[derive(Serialize, ToSchema)]
    #[schema(as = GetGroupNames::Response)]
    pub struct Response {
        /// Список названий групп отсортированный в алфавитном порядке
        #[schema(examples(json!(["ИС-214/23"])))]
        pub names: Vec<String>,
    }

    impl From<Vec<String>> for Response {
        fn from(names: Vec<String>) -> Self {
            Self { names }
        }
    }
}
