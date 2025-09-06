use crate::routes::schedule::schema::CacheStatus;
use crate::AppState;
use actix_web::{get, web};
use std::ops::Deref;

#[utoipa::path(responses(
    (status = OK, body = CacheStatus),
))]
#[get("/cache-status")]
pub async fn cache_status(app_state: web::Data<AppState>) -> CacheStatus {
    app_state
        .get_schedule_snapshot("eng_polytechnic")
        .await
        .unwrap()
        .deref()
        .into()
}
