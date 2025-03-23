pub mod sign_in {
    use crate::database::models::User;
    use crate::routes::schema::shared::{ErrorToHttpCode, IResponse};
    use crate::routes::schema::user;
    use actix_web::http::StatusCode;
    use serde::{Deserialize, Serialize};

    #[derive(Deserialize, Serialize)]
    pub struct Request {
        pub username: String,
        pub password: String,
    }

    pub type Response = IResponse<user::ResponseOk, ResponseErr>;

    #[derive(Serialize)]
    pub struct ResponseErr {
        code: ErrorCode,
    }

    #[derive(Serialize)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum ErrorCode {
        IncorrectCredentials,
        InvalidVkAccessToken,
    }

    pub trait ResponseExt {
        fn ok(user: &User) -> Self;
        fn err(code: ErrorCode) -> Response;
    }

    impl ResponseExt for Response {
        fn ok(user: &User) -> Self {
            IResponse(Ok(user::ResponseOk::from_user(&user)))
        }

        fn err(code: ErrorCode) -> Response {
            IResponse(Err(ResponseErr { code }))
        }
    }

    impl ErrorToHttpCode for ResponseErr {
        fn to_http_status_code(&self) -> StatusCode {
            StatusCode::NOT_ACCEPTABLE
        }
    }
}

pub mod sign_up {
    use crate::database::models::{User, UserRole};
    use crate::routes::schema::shared::{ErrorToHttpCode, IResponse};
    use crate::routes::schema::user;
    use actix_web::http::StatusCode;
    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize)]
    pub struct Request {
        pub username: String,
        pub password: String,
        pub group: String,
        pub role: UserRole,
        pub version: String,
    }

    pub type Response = IResponse<user::ResponseOk, ResponseErr>;

    #[derive(Serialize)]
    #[serde(rename_all = "camelCase")]
    pub struct ResponseOk {
        id: String,
        access_token: String,
        group: String,
    }

    #[derive(Serialize)]
    pub struct ResponseErr {
        code: ErrorCode,
    }

    #[derive(Serialize)]
    #[serde(rename_all = "SCREAMING_SNAKE_CASE")]
    pub enum ErrorCode {
        DisallowedRole,
        InvalidGroupName,
        UsernameAlreadyExists,
    }

    pub trait ResponseExt {
        fn ok(user: &User) -> Self;
        fn err(code: ErrorCode) -> Self;
    }

    impl ResponseExt for Response {
        fn ok(user: &User) -> Self {
            IResponse(Ok(user::ResponseOk::from_user(&user)))
        }

        fn err(code: ErrorCode) -> Response {
            Self(Err(ResponseErr { code }))
        }
    }

    impl ErrorToHttpCode for ResponseErr {
        fn to_http_status_code(&self) -> StatusCode {
            StatusCode::NOT_ACCEPTABLE
        }
    }
}
