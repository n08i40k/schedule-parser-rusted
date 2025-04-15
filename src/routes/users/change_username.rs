use self::schema::*;
use crate::app_state::AppState;
use crate::database::driver;
use crate::database::models::User;
use crate::extractors::base::SyncExtractor;
use crate::routes::schema::IntoResponseAsError;
use crate::utility::mutex::MutexScope;
use actix_web::{post, web};
use diesel::SaveChangesDsl;

#[utoipa::path(responses((status = OK)))]
#[post("/change-username")]
pub async fn change_username(
    app_state: web::Data<AppState>,
    user: SyncExtractor<User>,
    data: web::Json<Request>,
) -> ServiceResponse {
    let mut user = user.into_inner();

    if user.username == data.username {
        return ErrorCode::SameUsername.into_response();
    }

    if driver::users::get_by_username(&app_state, &data.username).is_ok() {
        return ErrorCode::AlreadyExists.into_response();
    }

    user.username = data.into_inner().username;

    if let Some(e) = app_state
        .database
        .scope(|conn| user.save_changes::<User>(conn))
        .err()
    {
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
    #[schema(as = ChangeUsername::Request)]
    pub struct Request {
        /// Новое имя.
        pub username: String,
    }

    #[derive(Clone, Serialize, ToSchema, StatusCode, Display, IntoResponseErrorNamed)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    #[schema(as = ChangeUsername::ErrorCode)]
    #[status_code = "actix_web::http::StatusCode::CONFLICT"]
    pub enum ErrorCode {
        /// Передано то же имя, что есть на данный момент.
        #[display("Passed the same name as it is at the moment.")]
        SameUsername,

        /// Пользователь с таким именем уже существует.
        #[display("A user with this name already exists.")]
        AlreadyExists,

        /// Ошибка на стороне сервера.
        #[display("Internal server error.")]
        #[status_code = "actix_web::http::StatusCode::INTERNAL_SERVER_ERROR"]
        InternalServerError,
    }
}
