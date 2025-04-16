use crate::app_state::AppState;
use crate::database;
use crate::database::models::FCM;
use crate::extractors::authorized_user::UserExtractor;
use crate::extractors::base::SyncExtractor;
use crate::utility::mutex::{MutexScope, MutexScopeAsync};
use actix_web::{HttpResponse, Responder, patch, web};
use diesel::{RunQueryDsl, SaveChangesDsl};
use firebase_messaging_rs::FCMClient;
use firebase_messaging_rs::topic::{TopicManagementError, TopicManagementSupport};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Params {
    pub token: String,
}

async fn get_fcm(
    app_state: &web::Data<AppState>,
    user_data: &UserExtractor<true>,
    token: String,
) -> Result<FCM, diesel::result::Error> {
    match user_data.fcm() {
        Some(fcm) => {
            let mut fcm = fcm.clone();
            fcm.token = token;

            Ok(fcm)
        }
        None => {
            let fcm = FCM {
                user_id: user_data.user().id.clone(),
                token,
                topics: vec![],
            };

            match app_state.database.scope(|conn| {
                diesel::insert_into(database::schema::fcm::table)
                    .values(&fcm)
                    .execute(conn)
            }) {
                Ok(_) => Ok(fcm),
                Err(e) => Err(e),
            }
        }
    }
}

#[utoipa::path(responses((status = OK)))]
#[patch("/set-token")]
pub async fn set_token(
    app_state: web::Data<AppState>,
    web::Query(params): web::Query<Params>,
    user_data: SyncExtractor<UserExtractor<true>>,
) -> impl Responder {
    let user_data = user_data.into_inner();

    // If token not changes - exit.
    if let Some(fcm) = user_data.fcm() {
        if fcm.token == params.token {
            return HttpResponse::Ok();
        }
    }

    let fcm = get_fcm(&app_state, &user_data, params.token.clone()).await;
    if let Err(e) = fcm {
        eprintln!("Failed to get FCM: {e}");
        return HttpResponse::Ok();
    }

    let mut fcm = fcm.ok().unwrap();

    // Add default topics.
    if !fcm.topics.contains(&Some("common".to_string())) {
        fcm.topics.push(Some("common".to_string()));
    }

    // Subscribe to default topics.
    if let Some(e) = app_state
        .fcm_client
        .as_ref()
        .unwrap()
        .async_scope(
            async |client: &mut FCMClient| -> Result<(), TopicManagementError> {
                let mut tokens: Vec<String> = Vec::new();
                tokens.push(fcm.token.clone());

                for topic in fcm.topics.clone() {
                    if let Some(topic) = topic {
                        client.register_tokens_to_topic(topic.clone(), tokens.clone()).await?;
                    }
                }

                Ok(())
            },
        )
        .await
        .err()
    {
        eprintln!("Failed to subscribe token to topic: {:?}", e);
        return HttpResponse::Ok();
    }

    // Write updates to db.
    if let Some(e) = app_state
        .database
        .scope(|conn| fcm.save_changes::<FCM>(conn))
        .err()
    {
        eprintln!("Failed to update FCM object: {e}");
    }

    HttpResponse::Ok()
}
