use crate::extractors::authorized_user;
use crate::state::AppState;
use crate::utility::req_auth::get_claims_from_req;
use actix_web::body::{BoxBody, EitherBody};
use actix_web::dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::{web, Error, HttpRequest, ResponseError};
use database::entity::sea_orm_active_enums::UserRole;
use database::query::Query;
use futures_util::future::LocalBoxFuture;
use std::future::{ready, Ready};
use std::ops::Deref;
use std::rc::Rc;
use std::sync::Arc;
use database::entity::UserType;

#[derive(Default, Clone)]
pub struct ServiceConfig {
    /// Allow service users to access endpoints.
    pub allow_service: bool,

    /// List of required roles to access endpoints.
    pub user_roles: Option<&'static [UserRole]>,
}

type ServiceKV = (Arc<[&'static str]>, Option<ServiceConfig>);

pub struct JWTAuthorizationBuilder {
    pub default_config: Option<ServiceConfig>,
    pub path_configs: Vec<ServiceKV>,
}

impl JWTAuthorizationBuilder {
    pub fn new() -> Self {
        JWTAuthorizationBuilder {
            default_config: Some(ServiceConfig::default()),
            path_configs: vec![],
        }
    }

    pub fn with_default(mut self, default: Option<ServiceConfig>) -> Self {
        self.default_config = default;
        self
    }

    pub fn add_paths(mut self, paths: impl AsRef<[&'static str]>, config: Option<ServiceConfig>) -> Self {
        self.path_configs.push((Arc::from(paths.as_ref()), config));
        self
    }

    pub fn build(self) -> JWTAuthorization {
        JWTAuthorization {
            default_config: Arc::new(self.default_config),
            path_configs: Arc::from(self.path_configs),
        }
    }
}

/// Middleware guard working with JWT tokens.
pub struct JWTAuthorization {
    pub default_config: Arc<Option<ServiceConfig>>,
    pub path_configs: Arc<[ServiceKV]>,
}

impl<S, B> Transform<S, ServiceRequest> for JWTAuthorization
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B, BoxBody>>;
    type Error = Error;
    type Transform = JWTAuthorizationMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(JWTAuthorizationMiddleware {
            service: Rc::new(service),
            default_config: self.default_config.clone(),
            path_configs: self.path_configs.clone(),
        }))
    }
}

pub struct JWTAuthorizationMiddleware<S> {
    service: Rc<S>,

    default_config: Arc<Option<ServiceConfig>>,
    path_configs: Arc<[ServiceKV]>,
}

impl<S, B> JWTAuthorizationMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    /// Checking the validity of the token.
    async fn check_authorization(
        req: &HttpRequest,
        allow_service_user: bool,
        required_user_roles: Option<&'static [UserRole]>,
    ) -> Result<(), authorized_user::Error> {
        let claims = get_claims_from_req(req).map_err(authorized_user::Error::from)?;

        let db = req
            .app_data::<web::Data<AppState>>()
            .unwrap()
            .get_database();

        let user_type = claims.user_type.unwrap_or(UserType::Default);

        match user_type {
            UserType::Default => {
                if let Some(required_user_roles) = required_user_roles {
                    let Ok(Some(user)) = Query::find_user_by_id(db, &claims.id).await else {
                        return Err(authorized_user::Error::NoUser);
                    };

                    if !required_user_roles.contains(&user.role) {
                        return Err(authorized_user::Error::InsufficientRights);
                    }

                    return Ok(());
                }

                match Query::is_user_exists_by_id(db, &claims.id).await {
                    Ok(true) => Ok(()),
                    _ => Err(authorized_user::Error::NoUser),
                }
            }
            UserType::Service => {
                if !allow_service_user {
                    return Err(authorized_user::Error::NonDefaultUserType);
                }

                match Query::is_service_user_exists_by_id(db, &claims.id).await {
                    Ok(true) => Ok(()),
                    _ => Err(authorized_user::Error::NoUser),
                }
            }
        }
    }

    fn find_config(
        current_path: &str,
        per_route: &[ServiceKV],
        default: &Option<ServiceConfig>,
    ) -> Option<ServiceConfig> {
        for (service_paths, config) in per_route {
            for service_path in service_paths.deref() {
                if !service_path.eq(&current_path) {
                    continue;
                }

                return config.clone();
            }
        }

        default.clone()
    }
}

impl<S, B> Service<ServiceRequest> for JWTAuthorizationMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B, BoxBody>>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let service = Rc::clone(&self.service);

        let Some(config) = Self::find_config(
            req.match_info().unprocessed(),
            &self.path_configs,
            &self.default_config,
        ) else {
            let fut = self.service.call(req);
            return Box::pin(async move { Ok(fut.await?.map_into_left_body()) });
        };

        let allow_service_user = config.allow_service;
        let required_user_roles = config.user_roles;

        Box::pin(async move {
            match Self::check_authorization(req.request(), allow_service_user, required_user_roles)
                .await
            {
                Ok(_) => {
                    let fut = service.call(req).await?;
                    Ok(fut.map_into_left_body())
                }
                Err(err) => Ok(ServiceResponse::new(
                    req.into_parts().0,
                    err.error_response().map_into_right_body(),
                )),
            }
        })
    }
}
