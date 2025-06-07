use crate::database::driver::users::UserSave;
use crate::database::models::User;
use crate::extractors::base::AsyncExtractor;
use crate::state::AppState;
use actix_web::{HttpResponse, Responder, post, web};

#[utoipa::path(responses(
    (status = OK),
    (status = INTERNAL_SERVER_ERROR)
))]
#[post("/update-callback/{version}")]
async fn update_callback(
    app_state: web::Data<AppState>,
    version: web::Path<String>,
    user: AsyncExtractor<User>,
) -> impl Responder {
    let mut user = user.into_inner();

    user.android_version = Some(version.into_inner());

    user.save(&app_state).await.unwrap();

    HttpResponse::Ok()
}
