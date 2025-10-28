use crate::utility::jwt;
use crate::utility::jwt::Claims;
use actix_web::http::header;
use actix_web::HttpRequest;

#[derive(Debug, PartialEq)]
pub enum Error {
    /// There is no Authorization header or cookie in the request.
    NoHeaderOrCookieFound,

    /// Unknown authorization type other than Bearer.
    UnknownAuthorizationType,

    /// Invalid or expired access token.
    InvalidAccessToken,
}

pub fn get_access_token_from_header(req: &HttpRequest) -> Result<String, Error> {
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

pub fn get_access_token_from_cookies(req: &HttpRequest) -> Result<String, Error> {
    let cookie = req
        .cookie("access_token")
        .ok_or(Error::NoHeaderOrCookieFound)?;

    Ok(cookie.value().to_string())
}

pub fn get_claims_from_req(req: &HttpRequest) -> Result<Claims, Error> {
    let access_token = match get_access_token_from_header(req) {
        Err(Error::NoHeaderOrCookieFound) => get_access_token_from_cookies(req)?,
        Err(error) => {
            return Err(error);
        }
        Ok(access_token) => access_token,
    };

    jwt::verify_and_decode(&access_token).map_err(|_| Error::InvalidAccessToken)
}
