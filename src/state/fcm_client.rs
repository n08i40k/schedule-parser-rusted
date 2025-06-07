use firebase_messaging_rs::FCMClient;
use std::env;
use tokio::sync::Mutex;

#[derive(Clone)]
pub struct FCMClientData;

impl FCMClientData {
    pub async fn new() -> Option<Mutex<FCMClient>> {
        match env::var("GOOGLE_APPLICATION_CREDENTIALS") {
            Ok(_) => Some(Mutex::new(FCMClient::new().await.unwrap())),
            Err(_) => None,
        }
    }
}
