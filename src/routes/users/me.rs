use crate::database::models::User;
use crate::extractors::base::SyncExtractor;
use actix_web::get;
use crate::routes::schema::user::UserResponse;

#[utoipa::path(responses((status = OK, body = UserResponse)))]
#[get("/me")]
pub async fn me(user: SyncExtractor<User>) -> UserResponse {
    user.into_inner().into()
}
