use crate::database::driver;
use crate::database::models::{User, UserRole};
use crate::routes::auth::schema;
use crate::{utility, AppState};
use actix_web::{post, web};
use objectid::ObjectId;
use web::Json;

#[post("/sign-up")]
pub async fn sign_up(
    data: Json<schema::sign_up::Request>,
    app_state: web::Data<AppState>,
) -> schema::sign_up::Response {
    use schema::sign_up::*;

    if data.role == UserRole::Admin {
        return Response::err(ErrorCode::DisallowedRole);
    }

    let schedule_opt = app_state.schedule.lock().unwrap();

    if let Some(schedule) = &*schedule_opt {
        if !schedule.data.groups.contains_key(&data.group) {
            return Response::err(ErrorCode::InvalidGroupName);
        }
    }

    if driver::users::contains_by_username(&app_state.database, data.username.clone()) {
        return Response::err(ErrorCode::UsernameAlreadyExists);
    }

    let id = ObjectId::new().unwrap().to_string();
    let access_token = utility::jwt::encode(&id);

    let user = User {
        id,
        username: data.username.clone(),
        password: bcrypt::hash(data.password.as_str(), bcrypt::DEFAULT_COST).unwrap(),
        vk_id: None,
        access_token,
        group: data.group.clone(),
        role: data.role.clone(),
        version: data.version.clone(),
    };

    driver::users::insert(&app_state.database, &user).unwrap();

    Response::ok(&user)
}

#[cfg(test)]
mod tests {
    use crate::app_state::app_state;
    use crate::database::driver;
    use crate::database::models::UserRole;
    use crate::routes::auth::schema;
    use crate::routes::auth::sign_up::sign_up;
    use crate::test_env::tests::{static_app_state, test_app, test_env};
    use actix_http::StatusCode;
    use actix_web::dev::ServiceResponse;
    use actix_web::http::Method;
    use actix_web::test;

    struct SignUpPartial {
        username: String,
        group: String,
        role: UserRole,
    }

    async fn sign_up_client(data: SignUpPartial) -> ServiceResponse {
        let app = test_app(app_state(), sign_up).await;

        let req = test::TestRequest::with_uri("/sign-up")
            .method(Method::POST)
            .set_json(schema::sign_up::Request {
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
        driver::users::delete_by_username(&app_state.database, "test::sign_up_valid".to_string());

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
            "test::sign_up_multiple".to_string(),
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
}
