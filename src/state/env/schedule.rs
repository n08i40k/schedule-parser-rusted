use std::env;

#[derive(Clone)]
pub struct ScheduleEnvData {
    /// Public link to the Yandex Disk folder the schedule files are uploaded to.
    #[cfg(not(test))]
    pub yandex_disk_url: String,

    pub auto_update: bool,
}

impl Default for ScheduleEnvData {
    fn default() -> Self {
        Self {
            #[cfg(not(test))]
            yandex_disk_url: env::var("SCHEDULE_YANDEX_DISK_URL")
                .expect("SCHEDULE_YANDEX_DISK_URL must be set"),
            auto_update: !env::var("SCHEDULE_DISABLE_AUTO_UPDATE")
                .is_ok_and(|v| v.eq("1") || v.eq("true")),
        }
    }
}
