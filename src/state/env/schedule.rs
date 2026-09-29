use std::env;

/// Public shared folder the schedule files are uploaded to.
#[cfg(not(test))]
#[derive(Clone)]
pub enum ScheduleSourceUrl {
    /// Public link to a Yandex Disk folder (`SCHEDULE_YANDEX_DISK_URL`).
    YandexDisk(String),

    /// Public link to a Dropbox folder (`SCHEDULE_DROPBOX_URL`).
    Dropbox(String),
}

#[cfg(not(test))]
impl ScheduleSourceUrl {
    fn from_env() -> Self {
        match (
            env::var("SCHEDULE_YANDEX_DISK_URL").ok(),
            env::var("SCHEDULE_DROPBOX_URL").ok(),
        ) {
            (Some(url), None) => Self::YandexDisk(url),
            (None, Some(url)) => Self::Dropbox(url),
            (Some(_), Some(_)) => {
                panic!("Only one of SCHEDULE_YANDEX_DISK_URL and SCHEDULE_DROPBOX_URL must be set")
            }
            (None, None) => {
                panic!("SCHEDULE_YANDEX_DISK_URL or SCHEDULE_DROPBOX_URL must be set")
            }
        }
    }
}

#[derive(Clone)]
pub struct ScheduleEnvData {
    #[cfg(not(test))]
    pub source_url: ScheduleSourceUrl,

    pub auto_update: bool,
}

impl Default for ScheduleEnvData {
    fn default() -> Self {
        Self {
            #[cfg(not(test))]
            source_url: ScheduleSourceUrl::from_env(),
            auto_update: !env::var("SCHEDULE_DISABLE_AUTO_UPDATE")
                .is_ok_and(|v| v.eq("1") || v.eq("true")),
        }
    }
}
