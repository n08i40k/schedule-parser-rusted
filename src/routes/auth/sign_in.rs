use crate::database::driver;
use crate::database::models::User;
use crate::routes::auth::schema::SignInErrCode::IncorrectCredentials;
use crate::routes::auth::schema::{SignInDto, SignInResult};
use crate::{AppState, utility};
use actix_web::{post, web};
use diesel::SaveChangesDsl;
use std::ops::DerefMut;
use web::Json;

#[post("/sign-in")]
pub async fn sign_in(data: Json<SignInDto>, app_state: web::Data<AppState>) -> Json<SignInResult> {
    let result = match driver::users::get_by_username(&app_state.database, data.username.clone()) {
        Ok(mut user) => match bcrypt::verify(&data.password, &user.password) {
            Ok(true) => {
                let mut lock = app_state.connection();
                let conn = lock.deref_mut();

                user.access_token = utility::jwt::encode(&user.id);

                user.save_changes::<User>(conn)
                    .expect("Failed to update user");

                SignInResult::ok(&user)
            }
            Ok(false) | Err(_) => SignInResult::err(IncorrectCredentials),
        },

        Err(_) => SignInResult::err(IncorrectCredentials),
    };

    Json(result)
}
