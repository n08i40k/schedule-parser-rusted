use crate::database::driver;
use crate::database::models::{User, FCM};
use crate::extractors::base::{AsyncExtractor, FromRequestAsync};
use crate::state::AppState;
use crate::utility::jwt;
use actix_macros::MiddlewareError;
use actix_web::body::BoxBody;
use actix_web::dev::Payload;
use actix_web::http::header;
use actix_web::{web, FromRequest, HttpRequest};
use derive_more::Display;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Display, MiddlewareError)]
#[status_code = "actix_web::http::StatusCode::UNAUTHORIZED"]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Error {
    /// There is no Authorization header or cookie in the request.
    #[display("No Authorization header or cookie found")]
    NoHeaderOrCookieFound,

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

fn get_access_token_from_header(req: &HttpRequest) -> Result<String, Error> {
    let header_value = req
        .headers()
        .get(header::AUTHORIZATION)
        .ok_or(Error::NoHeaderOrCookieFound)?
        .to_str()
        .map_err(|_| Error::NoHeaderOrCookieFound)?
        .to_string();

    let parts = header_value
        .split_once(' ')
        .ok_or(Error::UnknownAuthorizationType)?;

    if parts.0 != "Bearer" {
        Err(Error::UnknownAuthorizationType)
    } else {
        Ok(parts.1.to_string())
    }
}

fn get_access_token_from_cookies(req: &HttpRequest) -> Result<String, Error> {
    let cookie = req
        .cookie("access_token")
        .ok_or(Error::NoHeaderOrCookieFound)?;

    Ok(cookie.value().to_string())
}

/// User extractor from request with Bearer access token.
impl FromRequestAsync for User {
    type Error = actix_web::Error;

    async fn from_request_async(
        req: &HttpRequest,
        _payload: &mut Payload,
    ) -> Result<Self, Self::Error> {
        let access_token = match get_access_token_from_header(req) {
            Err(Error::NoHeaderOrCookieFound) => {
                get_access_token_from_cookies(req).map_err(|error| error.into_err())?
            }
            Err(error) => {
                return Err(error.into_err());
            }
            Ok(access_token) => access_token,
        };

        let user_id = jwt::verify_and_decode(&access_token)
            .map_err(|_| Error::InvalidAccessToken.into_err())?;

        let app_state = req.app_data::<web::Data<AppState>>().unwrap();

        driver::users::get(app_state, &user_id)
            .await
            .map_err(|_| Error::NoUser.into())
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
impl<const FCM: bool> FromRequestAsync for UserExtractor<{ FCM }> {
    type Error = actix_web::Error;

    async fn from_request_async(
        req: &HttpRequest,
        payload: &mut Payload,
    ) -> Result<Self, Self::Error> {
        let user = AsyncExtractor::<User>::from_request(req, payload)
            .await?
            .into_inner();

        let app_state = req.app_data::<web::Data<AppState>>().unwrap();

        Ok(Self {
            fcm: if FCM {
                driver::fcm::from_user(&app_state, &user).await.ok()
            } else {
                None
            },
            user,
        })
    }
}
