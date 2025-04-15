use crate::app_state::AppState;
use crate::database::models::User;
use crate::extractors::base::SyncExtractor;
use crate::utility::mutex::MutexScope;
use actix_web::{HttpResponse, Responder, post, web};
use diesel::SaveChangesDsl;

#[utoipa::path(responses(
    (status = OK),
    (status = INTERNAL_SERVER_ERROR)
))]
#[post("/update-callback/{version}")]
async fn update_callback(
    app_state: web::Data<AppState>,
    version: web::Path<String>,
    user: SyncExtractor<User>,
) -> impl Responder {
    let mut user = user.into_inner();

    user.version = version.into_inner();

    match app_state
        .database
        .scope(|conn| user.save_changes::<User>(conn))
    {
        Ok(_) => HttpResponse::Ok(),
        Err(e) => {
            eprintln!("Failed to update user: {}", e);
            HttpResponse::InternalServerError()
        }
    }
}
