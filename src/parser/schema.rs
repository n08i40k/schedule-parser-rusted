use chrono::{DateTime, Utc};
use derive_more::Display;
use serde::{Deserialize, Serialize, Serializer};
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::collections::HashMap;
use std::sync::Arc;
use utoipa::ToSchema;

#[derive(Clone, Hash, Debug, Serialize, Deserialize, ToSchema)]
pub struct LessonTime {
    /// Начало пары
    pub start: DateTime<Utc>,

    /// Конец пары
    pub end: DateTime<Utc>,
}

#[derive(Clone, Hash, PartialEq, Debug, Serialize_repr, Deserialize_repr, ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[repr(u8)]
pub enum LessonType {
    /// Обычная
    Default = 0,

    /// Допы
    Additional,

    /// Перемена
    Break,

    /// Консультация
    Consultation,

    /// Самостоятельная работа
    IndependentWork,

    /// Зачёт
    Exam,

    /// Зачет с оценкой
    ExamWithGrade,

    /// Экзамен
    ExamDefault,
}

#[derive(Clone, Hash, Debug, Serialize, Deserialize, ToSchema)]
pub struct LessonSubGroup {
    /// Номер подгруппы
    pub number: u8,

    /// Кабинет, если присутствует
    pub cabinet: Option<String>,

    /// Фио преподавателя
    pub teacher: String,
}

#[derive(Clone, Hash, Debug, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Lesson {
    /// Тип занятия
    #[serde(rename = "type")]
    pub lesson_type: LessonType,

    /// Индексы пар, если присутствуют
    pub default_range: Option<[u8; 2]>,

    /// Название занятия
    pub name: Option<String>,

    /// Начало и конец занятия
    pub time: LessonTime,

    /// Список подгрупп
    #[serde(rename = "subGroups")]
    pub subgroups: Option<Vec<LessonSubGroup>>,

    /// Группа, если это расписание для преподавателей
    pub group: Option<String>,
}

#[derive(Clone, Hash, Debug, Serialize, Deserialize, ToSchema)]
pub struct Day {
    /// День недели
    pub name: String,

    /// Адрес другого корпуса
    pub street: Option<String>,

    /// Дата
    pub date: DateTime<Utc>,

    /// Список пар в этот день
    pub lessons: Vec<Lesson>,
}

#[derive(Clone, Hash, Debug, Serialize, Deserialize, ToSchema)]
pub struct ScheduleEntry {
    /// Название группы или ФИО преподавателя
    pub name: String,

    /// Список из шести дней
    pub days: Vec<Day>,
}

#[derive(Clone)]
pub struct ParseResult {
    /// Список групп
    pub groups: HashMap<String, ScheduleEntry>,

    /// Список преподавателей
    pub teachers: HashMap<String, ScheduleEntry>,
}

#[derive(Debug, Display, Clone, ToSchema)]
pub enum ParseError {
    /// Ошибки связанные с чтением XLS файла.
    #[display("{}: Failed to read XLS file.", "_0")]
    #[schema(value_type = String)]
    BadXLS(Arc<calamine::XlsError>),

    /// Не найдено ни одного листа
    #[display("No work sheets found.")]
    NoWorkSheets,

    /// Отсутствуют данные об границах листа
    #[display("There is no data on work sheet boundaries.")]
    UnknownWorkSheetRange,

    /// Не удалось прочитать начало и конец пары из строки
    #[display("Failed to read lesson start and end times from string.")]
    GlobalTime,

    /// Не найдены начало и конец соответствующее паре
    #[display("No start and end times matching the lesson was found.")]
    LessonTimeNotFound,

    /// Не удалось прочитать индекс подгруппы
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
