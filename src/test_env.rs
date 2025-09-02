#[cfg(test)]
pub(crate) mod tests {
    use crate::state::{new_app_state, AppState};
    use actix_web::web;
    use log::info;
    use tokio::sync::OnceCell;

    pub fn test_env() {
        info!("Loading test environment file...");
        dotenvy::from_filename(".env.test.local")
            .or_else(|_| dotenvy::from_filename(".env.test"))
            .expect("Failed to load test environment file");
    }

    pub async fn test_app_state() -> web::Data<AppState> {
        let state = new_app_state().await.unwrap();

        state.clone()
    }

    pub async fn static_app_state() -> web::Data<AppState> {
        static STATE: OnceCell<web::Data<AppState>> = OnceCell::const_new();
        
        STATE.get_or_init(|| test_app_state()).await.clone()
    }
}
