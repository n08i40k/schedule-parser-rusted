use self::schema::*;
use crate::extractors::base::AsyncExtractor;
use crate::state::AppState;
use actix_web::{post, web};
use database::entity::User;
use database::query::Query;
use database::sea_orm::{ActiveModelTrait, IntoActiveModel, Set};
use std::ops::Deref;

#[utoipa::path(responses((status = OK)))]
#[post("/change-username")]
pub async fn change_username(
    app_state: web::Data<AppState>,
    user: AsyncExtractor<User>,
    data: web::Json<Request>,
) -> ServiceResponse {
    let user = user.into_inner();

    if user.username == data.username {
        return Ok(()).into();
    }

    let db = app_state.get_database();

    if Query::is_user_exists_by_username(db, &data.username)
        .await
        .unwrap()
    {
        return Err(ErrorCode::AlreadyExists).into();
    }

    let mut active_user = user.into_active_model();
    active_user.username = Set(data.into_inner().username);
    active_user.update(db).await.unwrap();

    Ok(()).into()
}

mod schema {
    use actix_macros::ErrResponse;
    use derive_more::Display;
    use serde::{Deserialize, Serialize};
    use utoipa::ToSchema;

    pub type ServiceResponse = crate::routes::schema::Response<(), ErrorCode>;

    #[derive(Serialize, Deserialize, ToSchema)]
    #[schema(as = ChangeUsername::Request)]
    pub struct Request {
        /// User name.
        pub username: String,
    }

    #[derive(Clone, Serialize, Display, ToSchema, ErrResponse)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    #[schema(as = ChangeUsername::ErrorCode)]
    #[status_code = "actix_web::http::StatusCode::CONFLICT"]
    pub enum ErrorCode {
        /// A user with this name already exists.
        #[display("A user with this name already exists.")]
        AlreadyExists,
    }
}
