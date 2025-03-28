use crate::app_state::AppState;
use crate::parser::schema::ScheduleEntry;
use chrono::{DateTime, Utc};
use serde::Serialize;
use std::collections::HashMap;
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
pub struct ScheduleView {
    etag: String,
    replacer_id: Option<String>,
    uploaded_at: DateTime<Utc>,
    downloaded_at: DateTime<Utc>,
    groups: HashMap<String, ScheduleEntry>,
    teachers: HashMap<String, ScheduleEntry>,
    updated_groups: Vec<Vec<i32>>,
    updated_teachers: Vec<Vec<i32>>,
}

pub enum Error {
    NoSchedule,
}

impl TryFrom<&AppState> for ScheduleView {
    type Error = Error;

    fn try_from(app_state: &AppState) -> Result<Self, Self::Error> {
        let schedule_lock = app_state.schedule.lock().unwrap();

        if let Some(schedule_ref) = schedule_lock.as_ref() {
            let schedule = schedule_ref.clone();

            Ok(Self {
                etag: schedule.etag,
                replacer_id: None,
                uploaded_at: schedule.updated_at,
                downloaded_at: schedule.parsed_at,
                groups: schedule.data.groups,
                teachers: schedule.data.teachers,
                updated_groups: vec![],
                updated_teachers: vec![],
            })
        } else {
            Err(Error::NoSchedule)
        }
    }
}
