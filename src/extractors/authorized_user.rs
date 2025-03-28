use crate::app_state::AppState;
use crate::database::driver;
use crate::database::models::User;
use crate::extractors::base::FromRequestSync;
use crate::utility::jwt;
use actix_macros::ResponseErrorMessage;
use actix_web::body::BoxBody;
use actix_web::dev::Payload;
use actix_web::http::header;
use actix_web::{HttpRequest, web};
use derive_more::Display;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

#[derive(Clone, Debug, Serialize, Deserialize, Display, ResponseErrorMessage)]
#[status_code = "actix_web::http::StatusCode::UNAUTHORIZED"]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Error {
    /// В запросе отсутствует заголовок Authorization
    #[display("No Authorization header found")]
    NoHeader,

    /// Неизвестный тип авторизации, отличающийся от Bearer
    #[display("Bearer token is required")]
    UnknownAuthorizationType,

    /// Токен не действителен
    #[display("Invalid or expired access token")]
    InvalidAccessToken,

    /// Пользователь привязанный к токену не найден в базе данных
    #[display("No user associated with access token")]
    NoUser,
}

impl Error {
    pub fn into_err(self) -> actix_web::Error {
        actix_web::Error::from(self)
    }
}

/// Экстрактор пользователя из запроса с токеном
impl FromRequestSync for User {
    type Error = actix_web::Error;

    fn from_request_sync(req: &HttpRequest, _: &mut Payload) -> Result<Self, Self::Error> {
        let authorization = req
            .headers()
            .get(header::AUTHORIZATION)
            .ok_or(Error::NoHeader.into_err())?
            .to_str()
            .map_err(|_| Error::NoHeader.into_err())?
            .to_string();

        let parts: Vec<&str> = authorization.split(' ').collect();

        if parts.len() != 2 || parts[0] != "Bearer" {
            return Err(Error::UnknownAuthorizationType.into_err());
        }

        let user_id = jwt::verify_and_decode(&parts[1].to_string())
            .map_err(|_| Error::InvalidAccessToken.into_err())?;

        let app_state = req.app_data::<web::Data<AppState>>().unwrap();

        driver::users::get(&app_state.database, &user_id).map_err(|_| Error::NoUser.into())
    }
}
