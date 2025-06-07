pub mod schedule;
pub mod telegram;
pub mod vk_id;
pub mod yandex_cloud;

pub use self::schedule::ScheduleEnvData;
pub use self::telegram::TelegramEnvData;
pub use self::vk_id::VkIdEnvData;
pub use self::yandex_cloud::YandexCloudEnvData;

#[derive(Default)]
pub struct AppEnv {
    pub schedule: ScheduleEnvData,
    pub telegram: TelegramEnvData,
    pub vk_id: VkIdEnvData,
    pub yandex_cloud: YandexCloudEnvData,
}
