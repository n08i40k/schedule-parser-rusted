use jsonwebtoken::errors::ErrorKind;
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    sub: i32,
    iis: String,
    jti: i32,
    app: i32,
}

#[derive(Debug, PartialEq)]
pub enum Error {
    Jwt(ErrorKind),
    InvalidSignature,
    InvalidToken,
    Expired,
    UnknownIssuer(String),
    UnknownType(i32),
    UnknownClientId(i32),
}

//noinspection SpellCheckingInspection
const VK_PUBLIC_KEY: &str = concat!(
    "-----BEGIN PUBLIC KEY-----\n",
    "MIICIjANBgkqhkiG9w0BAQEFAAOCAg8AMIICCgKCAgEAvsvJlhFX9Ju/pvCz1frB\n",
    "DgJs592VjdwQuRAmnlJAItyHkoiDIOEocPzgcUBTbDf1plDcTyO2RCkUt0pz0WK6\n",
    "6HNhpJyIfARjaWHeUlv4TpuHXAJJsBKklkU2gf1cjID+40sWWYjtq5dAkXnSJUVA\n",
    "UR+sq0lJ7GmTdJtAr8hzESqGEcSP15PTs7VUdHZ1nkC2XgkuR8KmKAUb388ji1Q4\n",
    "n02rJNOPQgd9r0ac4N2v/yTAFPXumO78N25bpcuWf5vcL9e8THk/U2zt7wf+aAWL\n",
    "748e0pREqNluTBJNZfmhC79Xx6GHtwqHyyduiqfPmejmiujNM/rqnA4e30Tg86Yn\n",
    "cNZ6vLJyF72Eva1wXchukH/aLispbY+EqNPxxn4zzCWaLKHG87gaCxpVv9Tm0jSD\n",
    "2es22NjrUbtb+2pAGnXbyDp2eGUqw0RrTQFZqt/VcmmSCE45FlcZMT28otrwG1ZB\n",
    "kZAb5Js3wLEch3ZfYL8sjhyNRPBmJBrAvzrd8qa3rdUjkC9sKyjGAaHu2MNmFl1Y\n",
    "JFQ3J54tGpkGgJjD7Kz3w0K6OiPDlVCNQN5sqXm24fCw85Pbi8SJiaLTp/CImrs1\n",
    "Z3nHW5q8hljA7OGmqfOP0nZS/5zW9GHPyepsI1rW6CympYLJ15WeNzePxYS5KEX9\n",
    "EncmkSD9b45ge95hJeJZteUCAwEAAQ==\n",
    "-----END PUBLIC KEY-----"
);

pub fn parse_vk_id(token_str: &str, client_id: i32) -> Result<i32, Error> {
    let dkey = DecodingKey::from_rsa_pem(VK_PUBLIC_KEY.as_bytes()).unwrap();

    match decode::<Claims>(token_str, &dkey, &Validation::new(Algorithm::RS256)) {
        Ok(token_data) => {
            let claims = token_data.claims;

            if claims.iis != "VK" {
                Err(Error::UnknownIssuer(claims.iis))
            } else if claims.jti != 21 {
                Err(Error::UnknownType(claims.jti))
            } else if claims.app != client_id {
                Err(Error::UnknownClientId(claims.app))
            } else {
                Ok(claims.sub)
            }
        }
        Err(err) => Err(match err.into_kind() {
            ErrorKind::InvalidToken => Error::InvalidToken,
            ErrorKind::InvalidSignature => Error::InvalidSignature,
            ErrorKind::InvalidAlgorithmName => Error::InvalidToken,
            ErrorKind::MissingRequiredClaim(_) => Error::InvalidToken,
            ErrorKind::ExpiredSignature => Error::Expired,
            ErrorKind::InvalidAlgorithm => Error::InvalidToken,
            ErrorKind::MissingAlgorithm => Error::InvalidToken,
            ErrorKind::Base64(_) => Error::InvalidToken,
            ErrorKind::Json(_) => Error::InvalidToken,
            ErrorKind::Utf8(_) => Error::InvalidToken,
            kind => Error::Jwt(kind),
        }),
    }
}
