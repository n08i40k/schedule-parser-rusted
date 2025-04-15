use crate::app_state::AppState;
use crate::database::driver;
use crate::database::models::{FCM, User};
use crate::extractors::base::{FromRequestSync, SyncExtractor};
use crate::utility::jwt;
use actix_macros::ResponseErrorMessage;
use actix_web::body::BoxBody;
use actix_web::dev::Payload;
use actix_web::http::header;
use actix_web::{FromRequest, HttpRequest, web};
use derive_more::Display;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

#[derive(Clone, Debug, Serialize, Deserialize, Display, ResponseErrorMessage)]
#[status_code = "actix_web::http::StatusCode::UNAUTHORIZED"]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Error {
    /// There is no Authorization header in the request.
    #[display("No Authorization header found")]
    NoHeader,

    /// Unknown authorization type other than Bearer.
    #[display("Bearer token is required")]
    UnknownAuthorizationType,

    /// Invalid or expired access token.
    #[display("Invalid or expired access token")]
    InvalidAccessToken,

    /// The user bound to the token is not found in the database.
    #[display("No user associated with access token")]
    NoUser,
}

impl Error {
    pub fn into_err(self) -> actix_web::Error {
        actix_web::Error::from(self)
    }
}

/// User extractor from request with Bearer access token.
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

        driver::users::get(&app_state, &user_id).map_err(|_| Error::NoUser.into())
    }
}

pub struct UserExtractor<const FCM: bool> {
    user: User,

    fcm: Option<FCM>,
}

impl<const FCM: bool> UserExtractor<{ FCM }> {
    pub fn user(&self) -> &User {
        &self.user
    }

    pub fn fcm(&self) -> &Option<FCM> {
        if !FCM {
            panic!("FCM marked as not required, but it has been requested")
        }

        &self.fcm
    }
}

/// Extractor of user and additional parameters from request with Bearer token.
impl<const FCM: bool> FromRequestSync for UserExtractor<{ FCM }> {
    type Error = actix_web::Error;

    fn from_request_sync(req: &HttpRequest, payload: &mut Payload) -> Result<Self, Self::Error> {
        let user = SyncExtractor::<User>::from_request(req, payload)
            .into_inner()?
            .into_inner();

        let app_state = req.app_data::<web::Data<AppState>>().unwrap();

        Ok(Self {
            fcm: if FCM {
                driver::fcm::from_user(&app_state, &user).ok()
            } else {
                None
            },
            user,
        })
    }
}
