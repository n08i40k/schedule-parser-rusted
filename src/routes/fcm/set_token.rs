use crate::database;
use crate::database::models::FCM;
use crate::extractors::authorized_user::UserExtractor;
use crate::extractors::base::AsyncExtractor;
use crate::state::AppState;
use actix_web::{HttpResponse, Responder, patch, web};
use diesel::{RunQueryDsl, SaveChangesDsl};
use firebase_messaging_rs::topic::TopicManagementSupport;
use serde::Deserialize;
use std::ops::DerefMut;

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

            match diesel::insert_into(database::schema::fcm::table)
                .values(&fcm)
                .execute(app_state.get_database().await.deref_mut())
            {
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
    user_data: AsyncExtractor<UserExtractor<true>>,
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

    fcm.save_changes::<FCM>(app_state.get_database().await.deref_mut())
        .unwrap();

    let fcm_client = app_state.get_fcm_client().await.unwrap();

    for topic in fcm.topics.clone() {
        if let Some(topic) = topic {
            if let Err(error) = fcm_client
                .register_token_to_topic(&*topic, &*fcm.token)
                .await
            {
                eprintln!("Failed to subscribe token to topic: {:?}", error);
                return HttpResponse::Ok();
            }
        }
    }

    HttpResponse::Ok()
}
