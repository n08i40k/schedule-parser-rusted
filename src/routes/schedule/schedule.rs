use crate::routes::schedule::schema::ScheduleView;
use crate::state::AppState;
use actix_web::{get, web};

#[utoipa::path(responses((status = OK, body = ScheduleView)))]
#[get("/")]
pub async fn schedule(app_state: web::Data<AppState>) -> ScheduleView {
    ScheduleView::from(&app_state).await
}
