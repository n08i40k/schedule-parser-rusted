use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::fmt::Write;

/// Server response to errors within Middleware.
#[derive(Serialize, Deserialize)]
pub struct MiddlewareError<T: Display> {
    code: T,
    message: String,
}

impl<T: Display + Serialize> MiddlewareError<T> {
    pub fn new(code: T) -> Self {
        let mut message = String::new();
        write!(&mut message, "{}", code).unwrap();

        Self { code, message }
    }
}
