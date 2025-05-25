use crate::app_state::{AppState, Schedule};
use crate::parser::schema::ScheduleEntry;
use actix_macros::{IntoResponseErrorNamed, ResponderJson, StatusCode};
use actix_web::web;
use chrono::{DateTime, Duration, Utc};
use derive_more::Display;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;

/// Response from schedule server.
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleView {
    /// ETag schedules on polytechnic server.
    etag: String,

    /// Schedule update date on polytechnic website.
    uploaded_at: DateTime<Utc>,

    /// Date last downloaded from the Polytechnic server.
    downloaded_at: DateTime<Utc>,

    /// Groups schedule.
    groups: HashMap<String, ScheduleEntry>,

    /// Teachers schedule.
    teachers: HashMap<String, ScheduleEntry>,
}

#[derive(Clone, Serialize, ToSchema, StatusCode, Display, IntoResponseErrorNamed)]
#[status_code = "actix_web::http::StatusCode::SERVICE_UNAVAILABLE"]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[schema(as = ScheduleShared::ErrorCode)]
pub enum ErrorCode {
    /// Schedules not yet parsed.
    #[display("Schedule not parsed yet.")]
    NoSchedule,
}

impl TryFrom<&web::Data<AppState>> for ScheduleView {
    type Error = ErrorCode;

    fn try_from(app_state: &web::Data<AppState>) -> Result<Self, Self::Error> {
        if let Some(schedule) = app_state.schedule.lock().unwrap().clone() {
            Ok(Self {
                etag: schedule.etag,
                uploaded_at: schedule.updated_at,
                downloaded_at: schedule.parsed_at,
                groups: schedule.data.groups,
                teachers: schedule.data.teachers,
            })
        } else {
            Err(ErrorCode::NoSchedule)
        }
    }
}

/// Cached schedule status.
#[derive(Serialize, Deserialize, ToSchema, ResponderJson)]
#[serde(rename_all = "camelCase")]
pub struct CacheStatus {
    /// Schedule hash.
    pub cache_hash: String,

    /// Whether the schedule reference needs to be updated.
    pub cache_update_required: bool,

    /// Last cache update date.
    pub last_cache_update: i64,

    /// Cached schedule update date.
    ///
    /// Determined by the polytechnic's server.
    pub last_schedule_update: i64,
}

impl CacheStatus {
    pub fn default() -> Self {
        CacheStatus {
            cache_hash: "0000000000000000000000000000000000000000".to_string(),
            cache_update_required: true,
            last_cache_update: 0,
            last_schedule_update: 0,
        }
    }
}

impl From<&web::Data<AppState>> for CacheStatus {
    fn from(value: &web::Data<AppState>) -> Self {
        let schedule_lock = value.schedule.lock().unwrap();
        let schedule = schedule_lock.as_ref().unwrap();

        CacheStatus::from(schedule)
    }
}

impl From<&Schedule> for CacheStatus {
    fn from(value: &Schedule) -> Self {
        Self {
            cache_hash: value.hash(),
            cache_update_required: (Utc::now() - value.fetched_at) > Duration::minutes(5),
            last_cache_update: value.fetched_at.timestamp(),
            last_schedule_update: value.updated_at.timestamp(),
        }
    }
}
