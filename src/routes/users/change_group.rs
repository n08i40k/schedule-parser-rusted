use self::schema::*;
use crate::extractors::base::AsyncExtractor;
use crate::state::AppState;
use actix_web::{post, web};
use database::entity::User;
use database::sea_orm::{ActiveModelTrait, IntoActiveModel, Set};

#[utoipa::path(responses((status = OK)))]
#[post("/change-group")]
pub async fn change_group(
    app_state: web::Data<AppState>,
    user: AsyncExtractor<User>,
    data: web::Json<Request>,
) -> ServiceResponse {
    let user = user.into_inner();

    if user
        .group
        .as_ref()
        .is_some_and(|group| group.eq(&data.group))
    {
        return Ok(()).into();
    }

    if !app_state
        .get_schedule_snapshot("eng_polytechnic")
        .await
        .unwrap()
        .data
        .groups
        .contains_key(&data.group)
    {
        return Err(ErrorCode::NotFound).into();
    }

    let mut active_user = user.clone().into_active_model();
    active_user.group = Set(Some(data.into_inner().group));

    active_user.update(app_state.get_database()).await.unwrap();

    Ok(()).into()
}

mod schema {
    use actix_macros::ErrResponse;
    use derive_more::Display;
    use serde::{Deserialize, Serialize};
    use utoipa::ToSchema;

    pub type ServiceResponse = crate::routes::schema::Response<(), ErrorCode>;

    #[derive(Deserialize, ToSchema)]
    #[schema(as = ChangeGroup::Request)]
    pub struct Request {
        // Group.
        pub group: String,
    }

    #[derive(Clone, Serialize, Display, ToSchema, ErrResponse)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    #[schema(as = ChangeGroup::ErrorCode)]
    #[status_code = "actix_web::http::StatusCode::CONFLICT"]
    pub enum ErrorCode {
        /// The required group does not exist.
        #[display("The required group does not exist.")]
        #[status_code = "actix_web::http::StatusCode::NOT_FOUND"]
        NotFound,
    }
}
