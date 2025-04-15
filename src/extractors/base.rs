use actix_web::dev::Payload;
use actix_web::{FromRequest, HttpRequest};
use futures_util::future::LocalBoxFuture;
use std::future::{Ready, ready};
use std::ops;

/// # Async extractor.

/// Asynchronous object extractor from a query.
pub struct AsyncExtractor<T>(T);

impl<T> AsyncExtractor<T> {
    #[allow(dead_code)]
    /// Retrieve the object extracted with the extractor.
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> ops::Deref for AsyncExtractor<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> ops::DerefMut for AsyncExtractor<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub trait FromRequestAsync: Sized {
    type Error: Into<actix_web::Error>;

    /// Asynchronous function for extracting data from a query.
    ///
    /// returns: Result<Self, Self::Error>
    ///
    /// # Examples
    ///
    /// ```
    /// struct User {
    ///     pub id: String,
    ///     pub username: String,
    /// }
    ///
    /// // TODO: Я вообще этот экстрактор не использую, нахуя мне тогда писать пример, если я не ебу как его использовать. Я забыл.
    ///
    /// #[get("/")]
    /// fn get_user_async(
    ///     user: web::AsyncExtractor<User>,
    /// ) -> web::Json<User> {
    ///     let user = user.into_inner();    
    ///
    ///     web::Json(user)
    /// }
    /// ```
    async fn from_request_async(req: HttpRequest, payload: Payload) -> Result<Self, Self::Error>;
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

/// # Sync extractor.

/// Synchronous object extractor from a query.
pub struct SyncExtractor<T>(T);

impl<T> SyncExtractor<T> {
    /// Retrieving an object extracted with the extractor.
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> ops::Deref for SyncExtractor<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> ops::DerefMut for SyncExtractor<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

pub trait FromRequestSync: Sized {
    type Error: Into<actix_web::Error>;

    /// Synchronous function for extracting data from a query.
    ///
    /// returns: Result<Self, Self::Error>
    ///
    /// # Examples
    ///
    /// ```
    /// struct User {
    ///     pub id: String,
    ///     pub username: String,
    /// }
    ///
    /// impl FromRequestSync for User {
    ///     type Error = actix_web::Error;
    ///
    ///     fn from_request_sync(req: &HttpRequest, _: &mut Payload) -> Result<Self, Self::Error> {
    ///         // do magic here.
    ///
    ///         Ok(User {
    ///             id: "qwerty".to_string(),
    ///             username: "n08i40k".to_string()
    ///         })
    ///     }
    /// }
    ///
    /// #[get("/")]
    /// fn get_user_sync(
    ///     user: web::SyncExtractor<User>,
    /// ) -> web::Json<User> {
    ///     let user = user.into_inner();    
    ///
    ///     web::Json(user)
    /// }
    /// ```
    fn from_request_sync(req: &HttpRequest, payload: &mut Payload) -> Result<Self, Self::Error>;
}

impl<T: FromRequestSync> FromRequest for SyncExtractor<T> {
    type Error = T::Error;
    type Future = Ready<Result<Self, Self::Error>>;

    fn from_request(req: &HttpRequest, payload: &mut Payload) -> Self::Future {
        ready(T::from_request_sync(req, payload).map(|res| Self(res)))
    }
}
