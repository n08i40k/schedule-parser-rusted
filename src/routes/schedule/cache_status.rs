use crate::AppState;
use crate::routes::schedule::schema::CacheStatus;
use actix_web::{get, web};

#[utoipa::path(responses(
    (status = OK, body = CacheStatus),
))]
#[get("/cache-status")]
pub async fn cache_status(app_state: web::Data<AppState>) -> CacheStatus {
    CacheStatus::from(&app_state).await.into()
}
