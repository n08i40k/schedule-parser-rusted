use self::schema::*;
use crate::app_state::AppState;
use crate::database::driver::users::UserSave;
use crate::database::models::User;
use crate::extractors::base::SyncExtractor;
use crate::routes::schema::IntoResponseAsError;
use crate::utility::mutex::MutexScope;
use actix_web::{post, web};

#[utoipa::path(responses((status = OK)))]
#[post("/change-group")]
pub async fn change_group(
    app_state: web::Data<AppState>,
    user: SyncExtractor<User>,
    data: web::Json<Request>,
) -> ServiceResponse {
    let mut user = user.into_inner();

    if user.group == data.group {
        return ErrorCode::SameGroup.into_response();
    }

    if let Some(e) = app_state.schedule.scope(|schedule| match schedule {
        Some(schedule) => {
            if schedule.data.groups.contains_key(&data.group) {
                None
            } else {
                Some(ErrorCode::NotFound)
            }
        }
        None => Some(ErrorCode::NoSchedule),
    }) {
        return e.into_response();
    }

    user.group = data.into_inner().group;

    if let Some(e) = user.save(&app_state).err() {
        eprintln!("Failed to update user: {e}");
        return ErrorCode::InternalServerError.into_response();
    }

    Ok(()).into()
}

mod schema {
    use actix_macros::{IntoResponseErrorNamed, StatusCode};
    use derive_more::Display;
    use serde::{Deserialize, Serialize};
    use utoipa::ToSchema;

    pub type ServiceResponse = crate::routes::schema::Response<(), ErrorCode>;

    #[derive(Serialize, Deserialize, ToSchema)]
    #[schema(as = ChangeGroup::Request)]
    pub struct Request {
        /// Group name.
        pub group: String,
    }

    #[derive(Clone, Serialize, ToSchema, StatusCode, Display, IntoResponseErrorNamed)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    #[schema(as = ChangeGroup::ErrorCode)]
    #[status_code = "actix_web::http::StatusCode::CONFLICT"]
    pub enum ErrorCode {
        /// Schedules have not yet been received.
        #[display("Schedule not parsed yet.")]
        #[status_code = "actix_web::http::StatusCode::SERVICE_UNAVAILABLE"]
        NoSchedule,

        /// Passed the same group name that is currently there.
        #[display("Passed the same group name as it is at the moment.")]
        SameGroup,

        /// The required group does not exist.
        #[display("The required group does not exist.")]
        #[status_code = "actix_web::http::StatusCode::NOT_FOUND"]
        NotFound,

        /// Server-side error.
        #[display("Internal server error.")]
        #[status_code = "actix_web::http::StatusCode::INTERNAL_SERVER_ERROR"]
        InternalServerError,
    }
}
