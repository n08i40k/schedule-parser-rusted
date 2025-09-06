use crate::extractors::base::AsyncExtractor;
use crate::routes::schema::user::UserResponse;
use actix_web::get;
use database::entity::User;

#[utoipa::path(responses((status = OK, body = UserResponse)))]
#[get("/me")]
pub async fn me(user: AsyncExtractor<User>) -> UserResponse {
    user.into_inner().into()
}
