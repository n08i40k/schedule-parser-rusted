pub mod schedule;
pub mod telegram;
pub mod vk_id;

#[cfg(not(test))]
pub mod yandex_cloud;

pub use self::schedule::ScheduleEnvData;
pub use self::telegram::TelegramEnvData;
pub use self::vk_id::VkIdEnvData;

#[cfg(not(test))]
pub use self::yandex_cloud::YandexCloudEnvData;

#[derive(Default)]
pub struct AppEnv {
    pub schedule: ScheduleEnvData,
    pub telegram: TelegramEnvData,
    pub vk_id: VkIdEnvData,
    
    #[cfg(not(test))]
    pub yandex_cloud: YandexCloudEnvData,
}
