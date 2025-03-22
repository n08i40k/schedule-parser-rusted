use chrono::DateTime;
use chrono::Duration;
use chrono::TimeZone;
use chrono::Utc;
use hmac::{Hmac, Mac};
use jwt::{SignWithKey, Token, VerifyWithKey};
use sha2::Sha256;
use std::collections::BTreeMap;
use std::env;
use std::mem::discriminant;
use std::sync::LazyLock;

static JWT_SECRET: LazyLock<Hmac<Sha256>> = LazyLock::new(|| {
    let secret = env::var("JWT_SECRET").expect("JWT_SECRET must be set");

    Hmac::new_from_slice(secret.as_bytes()).expect("Hmac::new_from_slice failed")
});

#[derive(Debug)]
pub enum VerifyError {
    JwtError(jwt::Error),
    InvalidSignature,
    InvalidToken,
    Expired,
}

impl PartialEq for VerifyError {
    fn eq(&self, other: &Self) -> bool {
        discriminant(self) == discriminant(other)
    }
}

pub fn verify_and_decode(token: &String) -> Result<String, VerifyError> {
    let jwt = &*JWT_SECRET;

    let result: Result<BTreeMap<String, String>, jwt::Error> = token.verify_with_key(jwt);

    match result {
        Ok(claims) => {
            let exp = claims.get("exp").unwrap();
            let exp_date = DateTime::from_timestamp(exp.parse::<i64>().unwrap(), 0)
                .expect("Failed to parse expiration time");

            if Utc::now() > exp_date {
                return Err(VerifyError::Expired);
            }

            Ok(claims.get("id").cloned().unwrap())
        }
        Err(err) => Err(match err {
            jwt::Error::InvalidSignature => VerifyError::InvalidSignature,
            jwt::Error::Format | jwt::Error::Base64(_) | jwt::Error::NoClaimsComponent => {
                VerifyError::InvalidToken
            }

            _ => VerifyError::JwtError(err),
        }),
    }
}

pub fn encode(id: &String) -> String {
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

    Token::new(header, claims)
        .sign_with_key(&*JWT_SECRET)
        .unwrap()
        .as_str()
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use dotenvy::dotenv;

    #[test]
    fn test_encode() {
        dotenv().unwrap();

        assert_eq!(encode(&"test".to_string()).is_empty(), false);
    }

    #[test]
    fn test_decode_invalid_token() {
        dotenv().unwrap();

        let token = "".to_string();
        let result = verify_and_decode(&token);

        assert!(result.is_err());
        assert_eq!(result.err().unwrap(), VerifyError::InvalidToken);
    }

    #[test]
    fn test_decode_invalid_signature() {
        dotenv().unwrap();

        let token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJleHAiOiIxODY4ODEyOTI4IiwiaWF0IjoiMTc0MjY2ODkyOCIsImlkIjoiNjdkY2M5YTk1MDdiMDAwMDc3Mjc0NGEyIn0.DQYFYF-3DoJgCLOVdAWa47nUaCJAh16DXj-ChNSSmWz".to_string();
        let result = verify_and_decode(&token);

        assert!(result.is_err());
        assert_eq!(result.err().unwrap(), VerifyError::InvalidToken);
    }

    #[test]
    fn test_decode_expired() {
        dotenv().unwrap();

        let token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJleHAiOiIxNjE2NTI2Mzc2IiwiaWF0IjoiMTQ5MDM4MjM3NiIsImlkIjoiNjdkY2M5YTk1MDdiMDAwMDc3Mjc0NGEyIn0.Qc2LbMJTvl2hWzDM2XyQv4m9lIqR84COAESQAieUxz8".to_string();
        let result = verify_and_decode(&token);

        assert!(result.is_err());
        assert_eq!(result.err().unwrap(), VerifyError::Expired);
    }

    #[test]
    fn test_decode_ok() {
        dotenv().unwrap();

        let token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJleHAiOiIxODY4ODEyOTI4IiwiaWF0IjoiMTc0MjY2ODkyOCIsImlkIjoiNjdkY2M5YTk1MDdiMDAwMDc3Mjc0NGEyIn0.DQYFYF-3DoJgCLOVdAWa47nUaCJAh16DXj-ChNSSmWw".to_string();
        let result = verify_and_decode(&token);

        assert!(result.is_ok());
    }
}
