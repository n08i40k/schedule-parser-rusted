use crate::routes::schema::user::UserResponse;
use crate::routes::users::by::schema::{ErrorCode, ServiceResponse};
use crate::state::AppState;
use actix_web::{get, web};
use database::query::Query;

#[utoipa::path(responses((status = OK, body = UserResponse)))]
#[get("/id/{id}")]
pub async fn by_id(app_state: web::Data<AppState>, path: web::Path<String>) -> ServiceResponse {
    let user_id = path.into_inner();

    let db = app_state.get_database();

    match Query::find_user_by_id(db, &user_id).await {
        Ok(Some(user)) => Ok(UserResponse::from(user)),
        _ => Err(ErrorCode::NotFound),
    }
    .into()
}

#[utoipa::path(responses((status = OK, body = UserResponse)))]
#[get("/telegram-id/{id}")]
pub async fn by_telegram_id(
    app_state: web::Data<AppState>,
    path: web::Path<i64>,
) -> ServiceResponse {
    let telegram_id = path.into_inner();

    let db = app_state.get_database();

    match Query::find_user_by_telegram_id(db, telegram_id).await {
        Ok(Some(user)) => Ok(UserResponse::from(user)),
        _ => Err(ErrorCode::NotFound),
    }
    .into()
}

mod schema {
    use crate::routes::schema::user::UserResponse;
    use actix_macros::ErrResponse;
    use derive_more::Display;
    use serde::Serialize;
    use utoipa::ToSchema;

    pub type ServiceResponse = crate::routes::schema::Response<UserResponse, ErrorCode>;

    #[derive(Clone, Serialize, Display, ToSchema, ErrResponse)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    #[schema(as = Users::By::ErrorCode)]
    pub enum ErrorCode {
        /// User not found.
        #[status_code = "actix_web::http::StatusCode::NOT_FOUND"]
        #[display("Required user not found.")]
        NotFound,
    }
}
