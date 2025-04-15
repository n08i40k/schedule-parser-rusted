use std::fmt::{Write};
use std::fmt::Display;
use serde::{Deserialize, Serialize};

/// Server response to errors within Middleware.
#[derive(Serialize, Deserialize)]
pub struct ResponseErrorMessage<T: Display> {
    code: T,
    message: String,
}

impl<T: Display + Serialize> ResponseErrorMessage<T> {
    pub fn new(code: T) -> Self {
        let mut message = String::new();
        write!(&mut message, "{}", code).unwrap();

        Self { code, message }
    }
}