use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_repr::{Deserialize_repr, Serialize_repr};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Clone, utoipa::ToSchema)]
pub struct LessonTime {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

#[derive(Serialize_repr, Deserialize_repr, Debug, PartialEq, Clone, utoipa::ToSchema)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
#[repr(u8)]
pub enum LessonType {
    Default = 0,     // Обычная
    Additional,      // Допы
    Break,           // Перемена
    Consultation,    // Консультация
    IndependentWork, // Самостоятельная работа
    Exam,            // Зачёт
    ExamWithGrade,   // Зачет с оценкой
    ExamDefault,     // Экзамен
}

#[derive( Serialize, Deserialize, Debug, Clone, utoipa::ToSchema)]
pub struct LessonSubGroup {
    /**
     * Номер подгруппы.
     */
    pub number: u8,

    /**
     * Кабинет, если присутствует.
     */
    pub cabinet: Option<String>,

    /**
     * Фио преподавателя.
     */
    pub teacher: String,
}

#[derive(Serialize, Deserialize, Debug, Clone, utoipa::ToSchema)]
#[serde(rename_all = "camelCase")]
pub struct Lesson {
    /**
     * Тип занятия.
     */
    #[serde(rename = "type")]
    pub lesson_type: LessonType,

    /**
     * Индексы пар, если присутствуют.
     */
    pub default_range: Option<[u8; 2]>,

    /**
     * Название занятия.
     */
    pub name: Option<String>,

    /**
     * Начало и конец занятия.
     */
    pub time: LessonTime,

    /**
     * Подгруппы.
     */
    #[serde(rename = "subGroups")]
    pub subgroups: Option<Vec<LessonSubGroup>>,

    /**
     * Группа, если это расписание для преподавателей.
     */
    pub group: Option<String>,
}

#[derive(Serialize, Deserialize, Debug, Clone, utoipa::ToSchema)]
pub struct Day {
    /**
     * День недели.
     */
    pub name: String,

    /**
     * Адрес другого корпуса.
     */
    pub street: Option<String>,

    /**
     * Дата.
     */
    pub date: DateTime<Utc>,

    /**
     * Список пар в этот день.
     */
    pub lessons: Vec<Lesson>,
}

#[derive(Clone, Serialize, Deserialize, Debug, utoipa::ToSchema)]
pub struct ScheduleEntry {
    /**
     * Название группы или ФИО преподавателя.
     */
    pub name: String,

    /**
     * Список из шести дней.
     */
    pub days: Vec<Day>,
}

#[derive(Clone)]
pub struct ParseResult {
    /**
     * Список групп.
     * Ключом является название группы.
     */
    pub groups: HashMap<String, ScheduleEntry>,

    /**
     * Список преподавателей.
     * Ключом является ФИО преподавателя.
     */
    pub teachers: HashMap<String, ScheduleEntry>,
}
