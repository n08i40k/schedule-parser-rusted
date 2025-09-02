use self::schema::*;
use crate::AppState;
use actix_web::{get, web};

#[utoipa::path(responses((status = OK, body = Response)))]
#[get("/teacher-names")]
pub async fn teacher_names(app_state: web::Data<AppState>) -> Response {
    let mut names: Vec<String> = app_state
        .get_schedule_snapshot("eng_polytechnic")
        .await
        .unwrap()
        .data
        .teachers
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
    #[schema(as = GetTeacherNames::Response)]
    pub struct Response {
        /// List of teacher names sorted alphabetically.
        #[schema(examples(json!(["Хомченко Н.Е."])))]
        pub names: Vec<String>,
    }
}
