use self::schema::*;
use crate::database::driver;
use crate::database::driver::users::UserSave;
use crate::database::models::User;
use crate::extractors::base::AsyncExtractor;
use crate::state::AppState;
use actix_web::{post, web};

#[utoipa::path(responses((status = OK)))]
#[post("/change-username")]
pub async fn change_username(
    app_state: web::Data<AppState>,
    user: AsyncExtractor<User>,
    data: web::Json<Request>,
) -> ServiceResponse {
    let mut user = user.into_inner();

    if user.username == data.username {
        return Ok(()).into();
    }

    if driver::users::get_by_username(&app_state, &data.username)
        .await
        .is_ok()
    {
        return Err(ErrorCode::AlreadyExists).into();
    }

    user.username = data.into_inner().username;
    user.save(&app_state).await.unwrap();

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
