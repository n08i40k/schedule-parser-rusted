pub mod schedule;
pub mod telegram;
pub mod vk_id;

pub use self::schedule::ScheduleEnvData;
pub use self::telegram::TelegramEnvData;
pub use self::vk_id::VkIdEnvData;

#[derive(Default)]
pub struct AppEnv {
    pub schedule: ScheduleEnvData,
    pub telegram: TelegramEnvData,
    pub vk_id: VkIdEnvData,
}
