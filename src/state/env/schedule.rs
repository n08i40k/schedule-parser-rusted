use std::env;

#[derive(Clone)]
pub struct ScheduleEnvData {
    #[cfg(not(test))]
    pub url: Option<String>,
    pub auto_update: bool,
}

impl Default for ScheduleEnvData {
    fn default() -> Self {
        Self {
            #[cfg(not(test))]
            url: env::var("SCHEDULE_INIT_URL").ok(),
            auto_update: !env::var("SCHEDULE_DISABLE_AUTO_UPDATE")
                .is_ok_and(|v| v.eq("1") || v.eq("true")),
        }
    }
}
