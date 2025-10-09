use crate::hasher::DigestHasher;
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use sha1::{Digest, Sha1};
use std::collections::HashMap;
use std::hash::Hash;
use std::sync::Arc;
use tokio_util::sync::CancellationToken;
use utoipa::ToSchema;

mod hasher;

// pub(crate) mod internal {
//     use super::{LessonBoundaries, LessonType};
//     use chrono::{DateTime, Utc};
//
//     /// Data cell storing the group name.
//     pub struct GroupCellInfo {
//         /// Column index.
//         pub column: u32,
//
//         /// Text in the cell.
//         pub name: String,
//     }
//
//     /// Data cell storing the line.
//     pub struct DayCellInfo {
//         /// Line index.
//         pub row: u32,
//
//         /// Column index.
//         pub column: u32,
//
//         /// Day name.
//         pub name: String,
//
//         /// Date of the day.
//         pub date: DateTime<Utc>,
//     }
//
//     /// Data on the time of lessons from the second column of the schedule.
//     pub struct BoundariesCellInfo {
//         /// Temporary segment of the lesson.
//         pub time_range: LessonBoundaries,
//
//         /// Type of lesson.
//         pub lesson_type: LessonType,
//
//         /// The lesson index.
//         pub default_index: Option<u32>,
//
//         /// The frame of the cell.
//         pub xls_range: ((u32, u32), (u32, u32)),
//     }
// }

/// The beginning and end of the lesson.
#[derive(Clone, Hash, Debug, Serialize, Deserialize, ToSchema)]
pub struct LessonBoundaries {
    /// The beginning of a lesson.
    pub start: DateTime<Utc>,

    /// The end of the lesson.
    pub end: DateTime<Utc>,
}

/// Type of lesson.
#[derive(Clone, Hash, PartialEq, Debug, Serialize_repr, Deserialize_repr, ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[repr(u8)]
pub enum LessonType {
    /// Обычная.
    Default = 0,

    /// Допы.
    Additional,

    /// Перемена.
    Break,

    /// Консультация.
    Consultation,

    /// Самостоятельная работа.
    IndependentWork,

    /// Зачёт.
    Exam,

    /// Зачёт с оценкой.
    ExamWithGrade,

    /// Экзамен.
    ExamDefault,

    /// Курсовой проект.
    CourseProject,

    /// Защита курсового проекта.
    CourseProjectDefense,

    /// Практическое занятие.
    Practice,
}

#[derive(Clone, Hash, Debug, Serialize, Deserialize, ToSchema)]
pub struct LessonSubGroup {
    /// Cabinet, if present.
    pub cabinet: Option<String>,

    /// Full name of the teacher.
    pub teacher: Option<String>,
}

#[derive(Clone, Hash, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Lesson {
    /// Type.
    #[serde(rename = "type")]
    pub lesson_type: LessonType,

    /// Lesson indexes, if present.
    pub range: Option<[u8; 2]>,

    /// Name.
    pub name: Option<String>,

    /// The beginning and end.
    pub time: LessonBoundaries,

    /// List of subgroups.
    #[serde(rename = "subgroups")]
    pub subgroups: Option<Vec<Option<LessonSubGroup>>>,

    /// Group name, if this is a schedule for teachers.
    pub group: Option<String>,
}

#[derive(Clone, Hash, Debug, Serialize, Deserialize, ToSchema)]
pub struct Day {
    /// Day of the week.
    pub name: String,

    /// Address of another corps.
    pub street: Option<String>,

    /// Date.
    pub date: DateTime<Utc>,

    /// List of lessons on this day.
    pub lessons: Vec<Lesson>,
}

#[derive(Clone, Hash, Debug, Serialize, Deserialize, ToSchema)]
pub struct ScheduleEntry {
    /// The name of the group or name of the teacher.
    pub name: String,

    /// List of six days.
    pub days: Vec<Day>,
}

#[derive(Clone)]
pub struct ParsedSchedule {
    /// List of groups.
    pub groups: HashMap<String, ScheduleEntry>,

    /// List of teachers.
    pub teachers: HashMap<String, ScheduleEntry>,
}

/// Represents a snapshot of the schedule parsed from an XLS file.
#[derive(Clone)]
pub struct ScheduleSnapshot {
    /// Timestamp when the Polytechnic website was queried for the schedule.
    pub fetched_at: DateTime<Utc>,

    /// Timestamp indicating when the schedule was last updated on the Polytechnic website.
    ///
    /// <note>
    /// This value is determined by the website's content and does not depend on the application.
    /// </note>
    pub updated_at: DateTime<Utc>,

    /// URL pointing to the XLS file containing the source schedule data.
    pub url: String,

    /// Parsed schedule data in the application's internal representation.
    pub data: ParsedSchedule,
}

impl ScheduleSnapshot {
    /// Converting the schedule data into a hash.
    /// ### Important!
    /// The hash does not depend on the dates.
    /// If the application is restarted, but the file with source schedule will remain unchanged, then the hash will not change.
    pub fn hash(&self) -> String {
        let mut hasher = DigestHasher::from(Sha1::new());

        self.data.teachers.iter().for_each(|e| e.hash(&mut hasher));
        self.data.groups.iter().for_each(|e| e.hash(&mut hasher));

        hasher.finalize()
    }

    /// Simply updates the value of [`ScheduleSnapshot::fetched_at`].
    /// Used for auto-updates.
    pub fn update(&mut self) {
        self.fetched_at = Utc::now();
    }
}

#[async_trait]
pub trait ScheduleProvider
where
    Self: Sync + Send,
{
    /// Returns ok when task has been canceled.
    /// Returns err when error appeared while trying to parse or download schedule
    async fn start_auto_update_task(
        &self,
        cancellation_token: CancellationToken,
    ) -> Result<(), Box<dyn std::error::Error + Send + Sync + 'static>>;

    async fn get_schedule(&self) -> Arc<ScheduleSnapshot>;
}
