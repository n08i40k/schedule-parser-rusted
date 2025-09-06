use actix_web::body::{BoxBody, EitherBody};
use actix_web::dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::http::header;
use actix_web::http::header::HeaderValue;
use actix_web::Error;
use futures_util::future::LocalBoxFuture;
use std::future::{ready, Ready};

/// Middleware to specify the encoding in the Content-Type header.
pub struct ContentTypeBootstrap;

impl<S, B> Transform<S, ServiceRequest> for ContentTypeBootstrap
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error>,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<EitherBody<B, BoxBody>>;
    type Error = Error;
    type Transform = ContentTypeMiddleware<S>;
    type InitError = ();
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(ContentTypeMiddleware { service }))
    }
}

pub struct ContentTypeMiddleware<S> {
    service: S,
}

impl<S, B> Service<ServiceRequest> for ContentTypeMiddleware<S>
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
        let fut = self.service.call(req);

        Box::pin(async move {
            let mut response = fut.await?;

            let headers = response.response_mut().headers_mut();

            if let Some(content_type) = headers.get("Content-Type")
                && content_type == "application/json"
            {
                headers.insert(
                    header::CONTENT_TYPE,
                    HeaderValue::from_static("application/json; charset=utf8"),
                );
            }

            Ok(response.map_into_left_body())
        })
    }
}
