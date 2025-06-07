use std::env;

#[derive(Clone)]
pub struct VkIdEnvData {
    pub client_id: i32,
    pub redirect_url: String,
}

impl Default for VkIdEnvData {
    fn default() -> Self {
        Self {
            client_id: env::var("VK_ID_CLIENT_ID")
                .expect("VK_ID_CLIENT_ID must be set")
                .parse()
                .expect("VK_ID_CLIENT_ID must be integer"),
            redirect_url: env::var("VK_ID_REDIRECT_URI").expect("VK_ID_REDIRECT_URI must be set"),
        }
    }
}
