use crate::extractors::base::FromRequestAsync;
use crate::state::AppState;
use crate::utility::req_auth;
use crate::utility::req_auth::get_claims_from_req;
use actix_macros::MiddlewareError;
use actix_web::body::BoxBody;
use actix_web::dev::Payload;
use actix_web::{web, HttpRequest};
use database::entity::{User, UserType};
use database::query::Query;
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

    /// Default user is required.
    #[display("Non-default user type is owning this access token")]
    #[status_code = "actix_web::http::StatusCode::FORBIDDEN"]
    NonDefaultUserType,

    /// The user bound to the token is not found in the database.
    #[display("No user associated with access token")]
    NoUser,

    /// User doesn't have required role.
    #[display("You don't have sufficient rights")]
    #[status_code = "actix_web::http::StatusCode::FORBIDDEN"]
    InsufficientRights,
}

impl From<req_auth::Error> for Error {
    fn from(value: req_auth::Error) -> Self {
        match value {
            req_auth::Error::NoHeaderOrCookieFound => Error::NoHeaderOrCookieFound,
            req_auth::Error::UnknownAuthorizationType => Error::UnknownAuthorizationType,
            req_auth::Error::InvalidAccessToken => Error::InvalidAccessToken,
        }
    }
}

/// User extractor from request with Bearer access token.
impl FromRequestAsync for User {
    type Error = Error;

    async fn from_request_async(
        req: &HttpRequest,
        _payload: &mut Payload,
    ) -> Result<Self, Self::Error> {
        let claims = get_claims_from_req(req).map_err(Error::from)?;

        if claims.user_type.unwrap_or(UserType::Default) != UserType::Default {
            return Err(Error::NonDefaultUserType);
        }

        let db = req
            .app_data::<web::Data<AppState>>()
            .unwrap()
            .get_database();

        match Query::find_user_by_id(db, &claims.id).await {
            Ok(Some(user)) => Ok(user),
            _ => Err(Error::NoUser),
        }
    }
}
