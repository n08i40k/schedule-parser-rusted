use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use serde_repr::{Deserialize_repr, Serialize_repr};

#[derive(Serialize, Deserialize, Debug)]
pub struct LessonTime {
    pub start: DateTime<Utc>,
    pub end: DateTime<Utc>,
}

#[derive(Serialize_repr, Deserialize_repr, Debug, PartialEq, Clone)]
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

#[derive(Serialize, Deserialize, Debug)]
pub struct LessonSubGroup {
    pub number: u8,

    pub cabinet: Option<String>,

    pub teacher: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Lesson {
    /**
     * Тип занятия
     */
    #[serde(rename = "type")]
    pub lesson_type: LessonType,

    /**
     * Индексы пар, если присутствуют
     */
    #[serde(rename = "defaultRange")]
    pub default_range: Option<[u8; 2]>,

    /**
     * Название занятия
     */
    pub name: Option<String>,

    /**
     * Начало и конец занятия
     */
    pub time: LessonTime,

    /**
     * Подгруппы
     */
    #[serde(rename = "subGroups")]
    pub subgroups: Option<Vec<LessonSubGroup>>,

    /**
     * Группа (только для расписания преподавателей)
     */
    pub group: Option<String>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Day {
    pub name: String,

    pub street: Option<String>,

    pub date: DateTime<Utc>,

    pub lessons: Vec<Lesson>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Group {
    pub name: String,

    pub days: Vec<Day>,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Schedule {
    #[serde(rename = "updatedAt")]
    pub updated_at: DateTime<Utc>,

    pub groups: HashMap<String, Group>,

    #[serde(rename = "updatedGroups")]
    pub updated_groups: Vec<Vec<usize>>,
}
