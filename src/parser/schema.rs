use chrono::{DateTime, Utc};
use derive_more::Display;
use serde::{Deserialize, Serialize, Serializer};
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::collections::HashMap;
use std::sync::Arc;
use utoipa::ToSchema;

/// The beginning and end of the lesson.
#[derive(Clone, Hash, Debug, Serialize, Deserialize, ToSchema)]
pub struct LessonTime {
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
}

#[derive(Clone, Hash, Debug, Serialize, Deserialize, ToSchema)]
pub struct LessonSubGroup {
    /// Index of subgroup.
    pub number: u8,

    /// Cabinet, if present.
    pub cabinet: Option<String>,

    /// Full name of the teacher.
    pub teacher: String,
}

#[derive(Clone, Hash, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Lesson {
    /// Type.
    #[serde(rename = "type")]
    pub lesson_type: LessonType,

    /// Lesson indexes, if present.
    pub default_range: Option<[u8; 2]>,

    /// Name.
    pub name: Option<String>,

    /// The beginning and end.
    pub time: LessonTime,

    /// List of subgroups.
    #[serde(rename = "subGroups")]
    pub subgroups: Option<Vec<LessonSubGroup>>,

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
pub struct ParseResult {
    /// List of groups.
    pub groups: HashMap<String, ScheduleEntry>,

    /// List of teachers.
    pub teachers: HashMap<String, ScheduleEntry>,
}

#[derive(Debug, Display, Clone, ToSchema)]
pub enum ParseError {
    /// Errors related to reading XLS file.
    #[display("{}: Failed to read XLS file.", "_0")]
    #[schema(value_type = String)]
    BadXLS(Arc<calamine::XlsError>),

    /// Not a single sheet was found.
    #[display("No work sheets found.")]
    NoWorkSheets,

    /// There are no data on the boundaries of the sheet.
    #[display("There is no data on work sheet boundaries.")]
    UnknownWorkSheetRange,

    /// Failed to read the beginning and end of the lesson from the line
    #[display("Failed to read lesson start and end times from string.")]
    GlobalTime,

    /// Not found the beginning and the end corresponding to the lesson.
    #[display("No start and end times matching the lesson was found.")]
    LessonTimeNotFound,

    /// Failed to read the subgroup index.
    #[display("Failed to read subgroup index.")]
    SubgroupIndexParsingFailed,
}

impl Serialize for ParseError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            ParseError::BadXLS(_) => serializer.serialize_str("BAD_XLS"),
            ParseError::NoWorkSheets => serializer.serialize_str("NO_WORK_SHEETS"),
            ParseError::UnknownWorkSheetRange => {
                serializer.serialize_str("UNKNOWN_WORK_SHEET_RANGE")
            }
            ParseError::GlobalTime => serializer.serialize_str("GLOBAL_TIME"),
            ParseError::LessonTimeNotFound => serializer.serialize_str("LESSON_TIME_NOT_FOUND"),
            ParseError::SubgroupIndexParsingFailed => {
                serializer.serialize_str("SUBGROUP_INDEX_PARSING_FAILED")
            }
        }
    }
}
