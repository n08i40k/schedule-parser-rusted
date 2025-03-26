use crate::app_state::AppState;
use crate::database::models::User;
use crate::extractors::base::SyncExtractor;
use actix_web::{HttpResponse, Responder, get, web};

#[get("/me")]
pub async fn me(user: SyncExtractor<User>, app_state: web::Data<AppState>) -> impl Responder {
    HttpResponse::Ok().json(user.into_inner())
}

mod schema {}
