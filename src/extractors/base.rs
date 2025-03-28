use actix_web::dev::Payload;
use actix_web::{FromRequest, HttpRequest};
use futures_util::future::LocalBoxFuture;
use std::future::{Ready, ready};

/// Асинхронный экстрактор объектов из запроса
pub struct AsyncExtractor<T>(T);

impl<T> AsyncExtractor<T> {
    #[allow(dead_code)]
    /// Получение объекта, извлечённого с помощью экстрактора
    pub fn into_inner(self) -> T {
        self.0
    }
}

pub trait FromRequestAsync: Sized {
    type Error: Into<actix_web::Error>;

    /// Асинхронная функция для извлечения данных из запроса
    async fn from_request_async(req: HttpRequest, payload: Payload) -> Result<Self, Self::Error>;
}

/// Реализация треита FromRequest для всех асинхронных экстракторов
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

/// Синхронный экстрактор объектов из запроса
pub struct SyncExtractor<T>(T);

impl<T> SyncExtractor<T> {
    /// Получение объекта, извлечённого с помощью экстрактора
    pub fn into_inner(self) -> T {
        self.0
    }
}

pub trait FromRequestSync: Sized {
    type Error: Into<actix_web::Error>;

    /// Синхронная функция для извлечения данных из запроса
    fn from_request_sync(req: &HttpRequest, payload: &mut Payload) -> Result<Self, Self::Error>;
}

/// Реализация треита FromRequest для всех синхронных экстракторов
impl<T: FromRequestSync> FromRequest for SyncExtractor<T> {
    type Error = T::Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future {
        ready(T::from_request_sync(req, payload).map(|res| Self(res)))
    }
}
