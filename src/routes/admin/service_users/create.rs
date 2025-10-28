use self::schema::*;
use crate::{utility, AppState};
use actix_web::{post, web};
use database::entity::{ActiveServiceUser, UserType};
use database::query::Query;
use database::sea_orm::{ActiveModelTrait, Set};
use objectid::ObjectId;
use web::Json;

#[utoipa::path(responses(
    (status = OK, body = Response),
))]
#[post("/create")]
pub async fn create(data_json: Json<Request>, app_state: web::Data<AppState>) -> ServiceResponse {
    let service_user =
        match Query::find_service_user_by_id(app_state.get_database(), &data_json.name)
            .await
            .expect("Failed to find service user by name")
        {
            Some(_) => return Err(ErrorCode::AlreadyExists).into(),
            None => {
                let new_user = ActiveServiceUser {
                    id: Set(ObjectId::new().unwrap().to_string()),
                    name: Set(data_json.name.clone()),
                };

                new_user
                    .insert(app_state.get_database())
                    .await
                    .expect("Failed to insert service user")
            }
        };

    let access_token = utility::jwt::encode(UserType::Service, &service_user.id);
    Ok(Response::new(access_token)).into()
}

mod schema {
    use actix_macros::{ErrResponse, OkResponse};
    use derive_more::Display;
    use serde::{Deserialize, Serialize};
    use utoipa::ToSchema;

    #[derive(Debug, Deserialize, Serialize, ToSchema)]
    #[serde(rename_all = "camelCase")]
    #[schema(as = ServiceUser::Create::Request)]
    pub struct Request {
        /// Service username.
        pub name: String,
    }

    #[derive(Serialize, ToSchema, OkResponse)]
    #[serde(rename_all = "camelCase")]
    #[schema(as = ServiceUser::Create::Response)]
    pub struct Response {
        access_token: String,
    }

    impl Response {
        pub fn new(access_token: String) -> Self {
            Self { access_token }
        }
    }

    pub type ServiceResponse = crate::routes::schema::Response<Response, ErrorCode>;

    #[derive(Clone, ToSchema, Display, ErrResponse, Serialize)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    #[status_code = "actix_web::http::StatusCode::UNAUTHORIZED"]
    #[schema(as = ServiceUser::Create::ErrorCode)]
    pub enum ErrorCode {
        #[display("Service user with that name already exists.")]
        AlreadyExists,
    }
}
