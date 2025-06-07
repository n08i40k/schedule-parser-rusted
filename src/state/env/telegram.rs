use std::env;

#[derive(Clone)]
pub struct TelegramEnvData {
    pub bot_id: i64,
    pub mini_app_host: String,
    pub test_dc: bool,
}

impl Default for TelegramEnvData {
    fn default() -> Self {
        let _self = Self {
            bot_id: env::var("TELEGRAM_BOT_ID")
                .expect("TELEGRAM_BOT_ID must be set")
                .parse()
                .expect("TELEGRAM_BOT_ID must be integer"),
            mini_app_host: env::var("TELEGRAM_MINI_APP_HOST")
                .expect("TELEGRAM_MINI_APP_HOST must be set"),
            test_dc: env::var("TELEGRAM_TEST_DC").is_ok_and(|v| v.eq("1") || v.eq("true")),
        };

        if _self.test_dc {
            log::warn!("Using test data-center of telegram!");
        }

        _self
    }
}
