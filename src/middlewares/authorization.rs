use crate::database::models::User;
use crate::extractors::authorized_user;
use crate::extractors::base::FromRequestSync;
use actix_web::body::{BoxBody, EitherBody};
use actix_web::dev::{Payload, Service, ServiceRequest, ServiceResponse, Transform, forward_ready};
use actix_web::{Error, HttpRequest, ResponseError};
use futures_util::future::LocalBoxFuture;
use std::future::{Ready, ready};

/// Middleware guard working with JWT tokens.
pub struct JWTAuthorization;

impl<S, B> Transform<S, ServiceRequest> for JWTAuthorization
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B, BoxBody>>;
    type Error = Error;
    type Transform = JWTAuthorizationMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(JWTAuthorizationMiddleware { service }))
    }
}

pub struct JWTAuthorizationMiddleware<S> {
    service: S,
}

impl<S, B> JWTAuthorizationMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    /// Checking the validity of the token.
    pub fn check_authorization(
        &self,
        req: &HttpRequest,
        payload: &mut Payload,
    ) -> Result<(), authorized_user::Error> {
        User::from_request_sync(req, payload)
            .map(|_| ())
            .map_err(|e| e.as_error::<authorized_user::Error>().unwrap().clone())
    }
}

impl<S, B> Service<ServiceRequest> for JWTAuthorizationMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B, BoxBody>>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let (http_req, mut payload) = req.into_parts();

        if let Err(err) = self.check_authorization(&http_req, &mut payload) {
            return Box::pin(async move {
                Ok(ServiceResponse::new(
                    http_req,
                    err.error_response().map_into_right_body(),
                ))
            });
        }

        let req = ServiceRequest::from_parts(http_req, payload);
        let fut = self.service.call(req);

        Box::pin(async move { Ok(fut.await?.map_into_left_body()) })
    }
}
