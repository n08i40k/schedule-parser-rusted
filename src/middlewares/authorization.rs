use crate::database::models::User;
use crate::extractors::authorized_user;
use crate::extractors::base::FromRequestAsync;
use actix_web::body::{BoxBody, EitherBody};
use actix_web::dev::{Payload, Service, ServiceRequest, ServiceResponse, Transform, forward_ready};
use actix_web::{Error, HttpRequest, ResponseError};
use futures_util::future::LocalBoxFuture;
use std::future::{Ready, ready};
use std::rc::Rc;

/// Middleware guard working with JWT tokens.
pub struct JWTAuthorization {
    /// List of ignored endpoints.
    pub ignore: &'static [&'static str],
}

impl Default for JWTAuthorization {
    fn default() -> Self {
        Self { ignore: &[] }
    }
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
            ignore: self.ignore,
        }))
    }
}

pub struct JWTAuthorizationMiddleware<S> {
    service: Rc<S>,
    /// List of ignored endpoints.
    ignore: &'static [&'static str],
}

impl<S, B> JWTAuthorizationMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    /// Checking the validity of the token.
    async fn check_authorization(req: &HttpRequest) -> Result<(), authorized_user::Error> {
        let mut payload = Payload::None;

        User::from_request_async(req, &mut payload)
            .await
            .map(|_| ())
            .map_err(|e| e.as_error::<authorized_user::Error>().unwrap().clone())
    }

    fn should_skip(&self, req: &ServiceRequest) -> bool {
        let path = req.match_info().unprocessed();

        self.ignore.iter().any(|ignore| {
            if !path.starts_with(ignore) {
                return false;
            }

            if let Some(other) = path.as_bytes().iter().nth(ignore.len()) {
                return ['?' as u8, '/' as u8].contains(other);
            }

            true
        })
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
        if self.should_skip(&req) {
            let fut = self.service.call(req);
            return Box::pin(async move { Ok(fut.await?.map_into_left_body()) });
        }

        let service = Rc::clone(&self.service);

        Box::pin(async move {
            match Self::check_authorization(req.request()).await {
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
