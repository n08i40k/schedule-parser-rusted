use self::schema::*;
use crate::AppState;
use crate::database::driver;
use crate::database::models::UserRole;
use crate::routes::auth::shared::{Error, parse_vk_id};
use actix_web::{post, web};
use rand::{Rng, rng};
use web::Json;

async fn sign_up(data: SignUpData, app_state: &web::Data<AppState>) -> Response {
    // If user selected forbidden role.
    if data.role == UserRole::Admin {
        return Response::err(ErrorCode::DisallowedRole);
    }

    // If specified group doesn't exist in schedule.
    let schedule_opt = app_state.schedule.lock().unwrap();

    if let Some(schedule) = &*schedule_opt {
        if !schedule.data.groups.contains_key(&data.group) {
            return Response::err(ErrorCode::InvalidGroupName);
        }
    }

    // If user with specified username already exists.
    if driver::users::contains_by_username(&app_state.database, &data.username) {
        return Response::err(ErrorCode::UsernameAlreadyExists);
    }

    // If user with specified VKID already exists.
    if let Some(id) = data.vk_id {
        if driver::users::contains_by_vk_id(&app_state.database, id) {
            return Response::err(ErrorCode::VkAlreadyExists);
        }
    }

    let user = data.to_user();
    driver::users::insert(&app_state.database, &user).unwrap();

    Response::ok(&user)
}

#[post("/sign-up")]
pub async fn sign_up_default(data_json: Json<Request>, app_state: web::Data<AppState>) -> Response {
    let data = data_json.into_inner();

    sign_up(
        SignUpData {
            username: data.username,
            password: data.password,
            vk_id: None,
            group: data.group,
            role: data.role,
            version: data.version,
        },
        &app_state,
    )
    .await
}

#[post("/sign-up-vk")]
pub async fn sign_up_vk(data_json: Json<vk::Request>, app_state: web::Data<AppState>) -> Response {
    let data = data_json.into_inner();

    match parse_vk_id(&data.access_token) {
        Ok(id) => {
            sign_up(
                SignUpData {
                    username: data.username,
                    password: rng()
                        .sample_iter(&rand::distr::Alphanumeric)
                        .take(16)
                        .map(char::from)
                        .collect(),
                    vk_id: Some(id),
                    group: data.group,
                    role: data.role,
                    version: data.version,
                },
                &app_state,
            )
            .await
        }
        Err(err) => {
            if err != Error::Expired {
                eprintln!("Failed to parse vk id token!");
                eprintln!("{:?}", err);
            }

            Response::err(ErrorCode::InvalidVkAccessToken)
        }
    }
}

mod schema {
    use crate::database::models::{User, UserRole};
    use crate::routes::schema::{ErrorToHttpCode, IResponse, user};
    use crate::utility;
    use actix_web::http::StatusCode;
    use objectid::ObjectId;
    use serde::{Deserialize, Serialize};

    /// WEB

    #[derive(Serialize, Deserialize)]
    pub struct Request {
        pub username: String,
        pub password: String,
        pub group: String,
        pub role: UserRole,
        pub version: String,
    }

    pub mod vk {
        use crate::database::models::UserRole;
        use serde::{Deserialize, Serialize};

        #[derive(Serialize, Deserialize)]
        #[serde(rename_all = "camelCase")]
        pub struct Request {
            pub access_token: String,
            pub username: String,
            pub group: String,
            pub role: UserRole,
            pub version: String,
        }
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
        InvalidVkAccessToken,
        VkAlreadyExists,
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

    /// Internal

    pub struct SignUpData {
        pub username: String,
        pub password: String,
        pub vk_id: Option<i32>,
        pub group: String,
        pub role: UserRole,
        pub version: String,
    }

    impl SignUpData {
        pub fn to_user(self) -> User {
            let id = ObjectId::new().unwrap().to_string();
            let access_token = utility::jwt::encode(&id);

            User {
                id,
                username: self.username,
                password: bcrypt::hash(self.password, bcrypt::DEFAULT_COST).unwrap(),
                vk_id: self.vk_id,
                access_token,
                group: self.group,
                role: self.role,
                version: self.version,
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::database::driver;
    use crate::database::models::UserRole;
    use crate::routes::auth::sign_up::schema::Request;
    use crate::routes::auth::sign_up::sign_up_default;
    use crate::test_env::tests::{static_app_state, test_app_state, test_env};
    use actix_web::http::StatusCode;
    use actix_web::dev::ServiceResponse;
    use actix_web::http::Method;
    use actix_web::test;
    use actix_test::test_app;

    struct SignUpPartial {
        username: String,
        group: String,
        role: UserRole,
    }

    async fn sign_up_client(data: SignUpPartial) -> ServiceResponse {
        let app = test_app(test_app_state(), sign_up_default).await;

        let req = test::TestRequest::with_uri("/sign-up")
            .method(Method::POST)
            .set_json(Request {
                username: data.username.clone(),
                password: "example".to_string(),
                group: data.group.clone(),
                role: data.role.clone(),
                version: "1.0.0".to_string(),
            })
            .to_request();

        test::call_service(&app, req).await
    }

    #[actix_web::test]
    async fn sign_up_valid() {
        // prepare

        test_env();

        let app_state = static_app_state();
        driver::users::delete_by_username(&app_state.database, &"test::sign_up_valid".to_string());

        // test

        let resp = sign_up_client(SignUpPartial {
            username: "test::sign_up_valid".to_string(),
            group: "ИС-214/23".to_string(),
            role: UserRole::Student,
        })
        .await;

        assert_eq!(resp.status(), StatusCode::OK);
    }

    #[actix_web::test]
    async fn sign_up_multiple() {
        // prepare

        test_env();

        let app_state = static_app_state();
        driver::users::delete_by_username(
            &app_state.database,
            &"test::sign_up_multiple".to_string(),
        );

        let create = sign_up_client(SignUpPartial {
            username: "test::sign_up_multiple".to_string(),
            group: "ИС-214/23".to_string(),
            role: UserRole::Student,
        })
        .await;

        assert_eq!(create.status(), StatusCode::OK);

        let resp = sign_up_client(SignUpPartial {
            username: "test::sign_up_multiple".to_string(),
            group: "ИС-214/23".to_string(),
            role: UserRole::Student,
        })
        .await;

        assert_eq!(resp.status(), StatusCode::NOT_ACCEPTABLE);
    }

    #[actix_web::test]
    async fn sign_up_invalid_role() {
        test_env();

        // test
        let resp = sign_up_client(SignUpPartial {
            username: "test::sign_up_invalid_role".to_string(),
            group: "ИС-214/23".to_string(),
            role: UserRole::Admin,
        })
        .await;

        assert_eq!(resp.status(), StatusCode::NOT_ACCEPTABLE);
    }

    #[actix_web::test]
    async fn sign_up_invalid_group() {
        test_env();

        // test
        let resp = sign_up_client(SignUpPartial {
            username: "test::sign_up_invalid_group".to_string(),
            group: "invalid_group".to_string(),
            role: UserRole::Student,
        })
        .await;

        assert_eq!(resp.status(), StatusCode::NOT_ACCEPTABLE);
    }
}
