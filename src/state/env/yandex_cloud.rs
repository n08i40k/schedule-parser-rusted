use std::env;

#[derive(Clone)]
pub struct YandexCloudEnvData {
    pub api_key: String,
    pub func_id: String,
}

impl Default for YandexCloudEnvData {
    fn default() -> Self {
        Self {
            api_key: env::var("YANDEX_CLOUD_API_KEY").expect("YANDEX_CLOUD_API_KEY must be set"),
            func_id: env::var("YANDEX_CLOUD_FUNC_ID").expect("YANDEX_CLOUD_FUNC_ID must be set"),
        }
    }
}
