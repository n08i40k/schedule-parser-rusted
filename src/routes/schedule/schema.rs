use crate::state::{AppState, ScheduleSnapshot};
use actix_macros::{OkResponse, ResponderJson};
use actix_web::web;
use schedule_parser::schema::ScheduleEntry;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ops::Deref;
use utoipa::ToSchema;

/// Response from schedule server.
#[derive(Serialize, ToSchema, OkResponse, ResponderJson)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleView {
    /// Url to xls file.
    url: String,

    /// Groups schedule.
    groups: HashMap<String, ScheduleEntry>,

    /// Teachers schedule.
    teachers: HashMap<String, ScheduleEntry>,
}

#[derive(Serialize, ToSchema, OkResponse)]
pub struct ScheduleEntryResponse(ScheduleEntry);

impl From<ScheduleEntry> for ScheduleEntryResponse {
    fn from(value: ScheduleEntry) -> Self {
        Self(value)
    }
}

impl ScheduleView {
    pub async fn from(app_state: &web::Data<AppState>) -> Self {
        let schedule = app_state.get_schedule_snapshot().await.clone();

        Self {
            url: schedule.url,
            groups: schedule.data.groups,
            teachers: schedule.data.teachers,
        }
    }
}

/// Cached schedule status.
#[derive(Serialize, Deserialize, ToSchema, ResponderJson, OkResponse)]
#[serde(rename_all = "camelCase")]
pub struct CacheStatus {
    /// Schedule hash.
    pub hash: String,

    /// Last cache update date.
    pub fetched_at: i64,

    /// Cached schedule update date.
    ///
    /// Determined by the polytechnic's server.
    pub updated_at: i64,
}

impl CacheStatus {
    pub async fn from(value: &web::Data<AppState>) -> Self {
        From::<&ScheduleSnapshot>::from(value.get_schedule_snapshot().await.deref())
    }
}

impl From<&ScheduleSnapshot> for CacheStatus {
    fn from(value: &ScheduleSnapshot) -> Self {
        Self {
            hash: value.hash(),
            fetched_at: value.fetched_at.timestamp(),
            updated_at: value.updated_at.timestamp(),
        }
    }
}
