use chrono::Duration;
use chrono::Utc;
use jsonwebtoken::errors::ErrorKind;
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode};
use serde::{Deserialize, Serialize};
use serde_with::DisplayFromStr;
use serde_with::serde_as;
use std::env;
use std::mem::discriminant;
use std::sync::LazyLock;

/// Key for token verification.
static DECODING_KEY: LazyLock<DecodingKey> = LazyLock::new(|| {
    let secret = env::var("JWT_SECRET").expect("JWT_SECRET must be set");

    DecodingKey::from_secret(secret.as_bytes())
});

/// Key for creating a signed token.
static ENCODING_KEY: LazyLock<EncodingKey> = LazyLock::new(|| {
    let secret = env::var("JWT_SECRET").expect("JWT_SECRET must be set");

    EncodingKey::from_secret(secret.as_bytes())
});

/// Token verification errors.
#[allow(dead_code)]
#[derive(Debug)]
pub enum Error {
    /// The token has a different signature.
    InvalidSignature,

    /// Token reading error.
    InvalidToken(ErrorKind),

    /// Token expired.
    Expired,
}

impl PartialEq for Error {
    fn eq(&self, other: &Self) -> bool {
        discriminant(self) == discriminant(other)
    }
}

/// The data the token holds.
#[serde_as]
#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    /// User account UUID.
    id: String,

    /// Token creation date.
    #[serde_as(as = "DisplayFromStr")]
    iat: u64,

    /// Token expiry date.
    #[serde_as(as = "DisplayFromStr")]
    exp: u64,
}

/// Token signing algorithm.
pub(crate) const DEFAULT_ALGORITHM: Algorithm = Algorithm::HS256;

/// Checking the token and extracting the UUID of the user account from it.
pub fn verify_and_decode(token: &String) -> Result<String, Error> {
    let mut validation = Validation::new(DEFAULT_ALGORITHM);

    validation.required_spec_claims.remove("exp");
    validation.validate_exp = false;

    let result = decode::<Claims>(&token, &*DECODING_KEY, &validation);

    match result {
        Ok(token_data) => {
            if token_data.claims.exp < Utc::now().timestamp().unsigned_abs() {
                Err(Error::Expired)
            } else {
                Ok(token_data.claims.id)
            }
        }
        Err(err) => Err(match err.into_kind() {
            ErrorKind::InvalidSignature => Error::InvalidSignature,
            ErrorKind::ExpiredSignature => Error::Expired,
            kind => Error::InvalidToken(kind),
        }),
    }
}

/// Creating a user token.
pub fn encode(id: &String) -> String {
    let header = Header {
        typ: Some(String::from("JWT")),
        ..Default::default()
    };

    let iat = Utc::now();
    let exp = iat + Duration::days(365 * 4);

    let claims = Claims {
        id: id.clone(),
        iat: iat.timestamp().unsigned_abs(),
        exp: exp.timestamp().unsigned_abs(),
    };

    jsonwebtoken::encode(&header, &claims, &*ENCODING_KEY).unwrap()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_env::tests::test_env;

    #[test]
    fn test_encode() {
        test_env();

        assert_eq!(encode(&"test".to_string()).is_empty(), false);
    }

    #[test]
    fn test_decode_invalid_token() {
        test_env();

        let token = "".to_string();
        let result = verify_and_decode(&token);

        assert!(result.is_err());
        assert_eq!(
            result.err().unwrap(),
            Error::InvalidToken(ErrorKind::InvalidToken)
        );
    }

    //noinspection SpellCheckingInspection
    #[test]
    fn test_decode_invalid_signature() {
        test_env();

        let token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJleHAiOiIxNjE2NTI2Mzc2IiwiaWF0IjoiMTQ5MDM4MjM3NiIsImlkIjoiNjdkY2M5YTk1MDdiMDAwMDc3Mjc0NGEyIn0.Qc2LbMJTvl2hWzDM2XyQv4m9lIqR84COAESQAieUxz8".to_string();
        let result = verify_and_decode(&token);

        assert!(result.is_err());
        assert_eq!(result.err().unwrap(), Error::InvalidSignature);
    }

    //noinspection SpellCheckingInspection
    #[test]
    fn test_decode_expired() {
        test_env();

        let token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpZCI6IjY3ZGNjOWE5NTA3YjAwMDA3NzI3NDRhMiIsImlhdCI6IjAiLCJleHAiOiIwIn0.GBsVYvnZIfHXt00t-qmAdUMyHSyWOBtC0Mrxwg1HQOM".to_string();
        let result = verify_and_decode(&token);

        assert!(result.is_err());
        assert_eq!(result.err().unwrap(), Error::Expired);
    }

    //noinspection SpellCheckingInspection
    #[test]
    fn test_decode_ok() {
        test_env();

        let token = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJpZCI6IjY3ZGNjOWE5NTA3YjAwMDA3NzI3NDRhMiIsImlhdCI6Ijk5OTk5OTk5OTkiLCJleHAiOiI5OTk5OTk5OTk5In0.o1vN-ze5iaJrnlHqe7WARXMBhhzjxTjTKkjlmTGEnOI".to_string();
        let result = verify_and_decode(&token);

        assert!(result.is_ok());
    }
}
