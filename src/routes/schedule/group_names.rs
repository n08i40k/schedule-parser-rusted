use self::schema::*;
use crate::AppState;
use actix_web::{get, web};

#[utoipa::path(responses((status = OK, body = Response)))]
#[get("/group-names")]
pub async fn group_names(app_state: web::Data<AppState>) -> Response {
    let mut names: Vec<String> = app_state
        .get_schedule_snapshot()
        .await
        .data
        .groups
        .keys()
        .cloned()
        .collect();

    names.sort();

    Response { names }
}

mod schema {
    use actix_macros::ResponderJson;
    use serde::Serialize;
    use utoipa::ToSchema;

    #[derive(Serialize, ToSchema, ResponderJson)]
    #[schema(as = GetGroupNames::Response)]
    pub struct Response {
        /// List of group names sorted in alphabetical order.
        #[schema(examples(json!(["ИС-214/23"])))]
        pub names: Vec<String>,
    }
}
