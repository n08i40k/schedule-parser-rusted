use chrono::{DateTime, Duration, Utc};
use hmac::{Hmac, Mac};
use jwt::{SignWithKey, Token, VerifyWithKey};
use sha2::Sha256;
use std::collections::BTreeMap;
use std::env;
use std::sync::LazyLock;

static JWT_SECRET: LazyLock<Hmac<Sha256>> = LazyLock::new(|| {
    let secret = env::var("JWT_SECRET").expect("JWT_SECRET must be set");

    Hmac::new_from_slice(secret.as_bytes()).expect("Hmac::new_from_slice failed")
});

#[derive(Debug)]
pub enum VerifyError {
    JwtError(jwt::Error),
    InvalidSignature,
    NoExpirationTag,
    Expired,
    NoId,
}

pub fn verify_and_decode(token: &String) -> Result<String, VerifyError> {
    let jwt = &*JWT_SECRET;

    let result: Result<BTreeMap<String, String>, jwt::Error> = token.verify_with_key(jwt);

    match result {
        Ok(claims) => match claims.get("exp") {
            None => Err(VerifyError::NoExpirationTag),
            Some(exp) => {
                let exp_date = DateTime::from_timestamp(exp.parse::<i64>().unwrap(), 0)
                    .expect("Failed to parse expiration time");

                if Utc::now() > exp_date {
                    return Err(VerifyError::Expired);
                }

                match claims.get("id").cloned() {
                    None => Err(VerifyError::NoId),
                    Some(id) => Ok(id),
                }
            }
        },
        Err(err) => Err(match err {
            jwt::Error::InvalidSignature => VerifyError::InvalidSignature,

            _ => VerifyError::JwtError(err),
        }),
    }
}

pub fn encode(id: &String) -> Result<String, jwt::Error> {
    let header = jwt::Header {
        type_: Some(jwt::header::HeaderType::JsonWebToken),
        ..Default::default()
    };

    let mut claims = BTreeMap::new();

    let iat = Utc::now();
    let exp = iat + Duration::days(365 * 4);

    let iat_str = iat.timestamp().to_string();
    let exp_str = exp.timestamp().to_string();

    claims.insert("id", id.as_str());
    claims.insert("iat", iat_str.as_str());
    claims.insert("exp", exp_str.as_str());

    match Token::new(header, claims).sign_with_key(&*JWT_SECRET) {
        Ok(token) => Ok(token.as_str().to_string()),
        Err(err) => Err(err),
    }
}
