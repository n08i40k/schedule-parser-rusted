use crate::database::models::User;
use crate::routes::auth::schema::SignInErrCode::IncorrectCredentials;
use crate::routes::auth::schema::{SignInDto, SignInResult};
use crate::AppState;
use actix_web::{post, web};
use diesel::{ExpressionMethods, QueryDsl, RunQueryDsl, SelectableHelper};
use std::ops::DerefMut;
use web::Json;

#[post("/sign-in")]
pub async fn sign_in(data: Json<SignInDto>, app_state: web::Data<AppState>) -> Json<SignInResult> {
    use crate::database::schema::users::dsl::*;

    match {
        let mut lock = app_state.database.lock().unwrap();
        let connection = lock.deref_mut();

        users
            .filter(username.eq(data.username.clone()))
            .select(User::as_select())
            .first(connection)
    } {
        Ok(user) => Json(SignInResult::ok(&user)),
        Err(_) => Json(SignInResult::err(IncorrectCredentials)),
    }
}
