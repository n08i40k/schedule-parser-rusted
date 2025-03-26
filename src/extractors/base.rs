use actix_web::dev::Payload;
use actix_web::{FromRequest, HttpRequest};
use futures_util::future::LocalBoxFuture;
use std::future::{Ready, ready};

pub trait FromRequestAsync: Sized {
    type Error: Into<actix_web::Error>;

    async fn from_request_async(req: HttpRequest, payload: Payload) -> Result<Self, Self::Error>;
}

pub struct AsyncExtractor<T>(T);

impl<T> AsyncExtractor<T> {
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T: FromRequestAsync> FromRequest for AsyncExtractor<T> {
    type Error = T::Error;
    type Future = LocalBoxFuture<'static, Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future {
        let req = req.clone();
        let payload = payload.take();
        Box::pin(async move {
            T::from_request_async(req, payload)
                .await
                .map(|res| Self(res))
        })
    }
}

pub trait FromRequestSync: Sized {
    type Error: Into<actix_web::Error>;

    fn from_request_sync(req: &HttpRequest, payload: &mut Payload) -> Result<Self, Self::Error>;
}

pub struct SyncExtractor<T>(T);

impl<T> SyncExtractor<T> {
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T: FromRequestSync> FromRequest for SyncExtractor<T> {
    type Error = T::Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future {
        ready(T::from_request_sync(req, payload).map(|res| Self(res)))
    }
}
