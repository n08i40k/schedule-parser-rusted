use crate::app_state::{AppState, Schedule};
use crate::parser::schema::ScheduleEntry;
use actix_macros::{IntoResponseErrorNamed, ResponderJson, StatusCode};
use actix_web::web;
use chrono::{DateTime, Duration, Utc};
use derive_more::Display;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;

/// Ответ от сервера с расписаниями
#[derive(Serialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct ScheduleView {
    /// ETag расписания на сервере политехникума
    etag: String,
    
    /// Дата обновления расписания на сайте политехникума
    uploaded_at: DateTime<Utc>,
    
    /// Дата последнего скачивания расписания с сервера политехникума
    downloaded_at: DateTime<Utc>,
    
    /// Расписание групп
    groups: HashMap<String, ScheduleEntry>,
    
    /// Расписание преподавателей
    teachers: HashMap<String, ScheduleEntry>,
}

#[derive(Clone, Serialize, ToSchema, StatusCode, Display, IntoResponseErrorNamed)]
#[status_code = "actix_web::http::StatusCode::SERVICE_UNAVAILABLE"]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[schema(as = ScheduleShared::ErrorCode)]
pub enum ErrorCode {
    /// Расписания ещё не получены
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

/// Статус кешированного расписаний
#[derive(Serialize, Deserialize, ToSchema, ResponderJson)]
#[serde(rename_all = "camelCase")]
pub struct CacheStatus {
    /// Хеш расписаний
    pub cache_hash: String,
    
    /// Требуется ли обновить ссылку на расписание
    pub cache_update_required: bool,
    
    /// Дата последнего обновления кеша 
    pub last_cache_update: i64,
    
    /// Дата обновления кешированного расписания
    /// 
    /// Определяется сервером политехникума
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
            cache_update_required: (value.fetched_at - Utc::now()) > Duration::minutes(5),
            last_cache_update: value.fetched_at.timestamp(),
            last_schedule_update: value.updated_at.timestamp(),
        }
    }
}
