use crate::AppState;
use crate::routes::schedule::schema::CacheStatus;
use actix_web::{get, web};

#[utoipa::path(responses(
    (status = OK, body = CacheStatus),
))]
#[get("/cache-status")]
pub async fn get_cache_status(app_state: web::Data<AppState>) -> CacheStatus {
    // Prevent thread lock
    let has_schedule = app_state
        .schedule
        .lock()
        .as_ref()
        .map(|res| res.is_some())
        .unwrap();

    match has_schedule {
        true => CacheStatus::from(&app_state),
        false => CacheStatus::default(),
    }
    .into()
}
