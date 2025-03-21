use crate::database::models::User;
use serde::{Deserialize, Serialize, Serializer};

#[derive(Deserialize)]
pub struct SignInDto {
    pub username: String,
    pub password: String,
}

pub struct SignInResult(Result<SignInOk, SignInErr>);

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SignInOk {
    id: String,
    access_token: String,
    group: String,
}

#[derive(Serialize)]
pub struct SignInErr {
    code: SignInErrCode,
}

#[derive(Serialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum SignInErrCode {
    IncorrectCredentials,
    InvalidVkAccessToken,
}

impl SignInResult {
    pub fn ok(user: &User) -> Self {
        Self(Ok(SignInOk {
            id: user.id.clone(),
            access_token: user.access_token.clone(),
            group: user.group.clone(),
        }))
    }

    pub fn err(code: SignInErrCode) -> SignInResult {
        Self(Err(SignInErr { code }))
    }
}

impl Serialize for SignInResult {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match &self.0 {
            Ok(ok) => serializer.serialize_some(&ok),
            Err(err) => serializer.serialize_some(&err),
        }
    }
}
