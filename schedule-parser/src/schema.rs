use chrono::{DateTime, Utc};
use derive_more::{Display, Error};
use serde::{Deserialize, Serialize, Serializer};
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::collections::HashMap;
use std::sync::Arc;
use utoipa::ToSchema;

pub(crate) mod internal {
    use crate::schema::{LessonBoundaries, LessonType};
    use chrono::{DateTime, Utc};

    /// Data cell storing the group name.
    pub struct GroupCellInfo {
        /// Column index.
        pub column: u32,

        /// Text in the cell.
        pub name: String,
    }

    /// Data cell storing the line.
    pub struct DayCellInfo {
        /// Line index.
        pub row: u32,

        /// Column index.
        pub column: u32,

        /// Day name.
        pub name: String,

        /// Date of the day.
        pub date: DateTime<Utc>,
    }

    /// Data on the time of lessons from the second column of the schedule.
    pub struct BoundariesCellInfo {
        /// Temporary segment of the lesson.
        pub time_range: LessonBoundaries,

        /// Type of lesson.
        pub lesson_type: LessonType,

        /// The lesson index.
        pub default_index: Option<u32>,

        /// The frame of the cell.
        pub xls_range: ((u32, u32), (u32, u32)),
    }
}

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
    pub time: LessonBoundaries,

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

#[derive(Clone, Debug, Display, Error, ToSchema)]
#[display("row {row}, column {column}")]
pub struct ErrorCellPos {
    pub row: u32,
    pub column: u32,
}

#[derive(Clone, Debug, Display, Error, ToSchema)]
#[display("'{data}' at {pos}")]
pub struct ErrorCell {
    pub pos: ErrorCellPos,
    pub data: String,
}

impl ErrorCell {
    pub fn new(row: u32, column: u32, data: String) -> Self {
        Self {
            pos: ErrorCellPos { row, column },
            data,
        }
    }
}

#[derive(Clone, Debug, Display, Error, ToSchema)]
pub enum ParseError {
    /// Errors related to reading XLS file.
    #[display("{_0:?}: Failed to read XLS file.")]
    #[schema(value_type = String)]
    BadXLS(Arc<calamine::XlsError>),

    /// Not a single sheet was found.
    #[display("No work sheets found.")]
    NoWorkSheets,

    /// There are no data on the boundaries of the sheet.
    #[display("There is no data on work sheet boundaries.")]
    UnknownWorkSheetRange,

    /// Failed to read the beginning and end of the lesson from the cell
    #[display("Failed to read lesson start and end from {_0}.")]
    LessonBoundaries(ErrorCell),

    /// Not found the beginning and the end corresponding to the lesson.
    #[display("No start and end times matching the lesson (at {_0}) was found.")]
    LessonTimeNotFound(ErrorCellPos),
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
            ParseError::LessonBoundaries(_) => serializer.serialize_str("GLOBAL_TIME"),
            ParseError::LessonTimeNotFound(_) => serializer.serialize_str("LESSON_TIME_NOT_FOUND"),
        }
    }
}
