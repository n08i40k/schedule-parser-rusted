use crate::LessonParseResult::{Lessons, Street};
use crate::schema::LessonType::Break;
use crate::schema::internal::{BoundariesCellInfo, DayCellInfo, GroupCellInfo};
use crate::schema::{
    Day, ErrorCell, ErrorCellPos, Lesson, LessonBoundaries, LessonSubGroup, LessonType, ParseError,
    ParseResult, ScheduleEntry,
};
use crate::worksheet::WorkSheet;
use calamine::{Reader, Xls, open_workbook_from_rs};
use chrono::{DateTime, Duration, NaiveDate, NaiveTime, Utc};
use regex::Regex;
use std::collections::HashMap;
use std::io::Cursor;
use std::sync::LazyLock;

mod macros;
pub mod schema;
mod worksheet;

/// Obtaining a "skeleton" schedule from the working sheet.
fn parse_skeleton(
    worksheet: &WorkSheet,
) -> Result<(Vec<DayCellInfo>, Vec<GroupCellInfo>), ParseError> {
    let mut groups: Vec<GroupCellInfo> = Vec::new();
    let mut days: Vec<DayCellInfo> = Vec::new();

    let worksheet_start = worksheet.start().ok_or(ParseError::UnknownWorkSheetRange)?;
    let worksheet_end = worksheet.end().ok_or(ParseError::UnknownWorkSheetRange)?;

    let mut row = worksheet_start.0;

    while row < worksheet_end.0 {
        row += 1;

        let day_full_name = or_continue!(worksheet.get_string_from_cell(row, 0));

        // parse groups row when days column will found
        if groups.is_empty() {
            // переход на предыдущую строку
            row -= 1;

            for column in (worksheet_start.1 + 2)..=worksheet_end.1 {
                groups.push(GroupCellInfo {
                    column,
                    name: or_continue!(worksheet.get_string_from_cell(row, column)),
                });
            }

            // возврат на текущую строку
            row += 1;
        }

        let (day_name, day_date) = {
            let space_index = day_full_name.find(' ').unwrap();

            let name = day_full_name[..space_index].to_string();

            let date_slice = &day_full_name[space_index + 1..];
            let date = or_break!(NaiveDate::parse_from_str(date_slice, "%d.%m.%Y").ok())
                .and_time(NaiveTime::default())
                .and_utc();

            (name, date)
        };

        days.push(DayCellInfo {
            row,
            column: 0,
            name: day_name,
            date: day_date,
        });
    }

    Ok((days, groups))
}

/// The result of obtaining a lesson from the cell.
enum LessonParseResult {
    /// List of lessons long from one to two.
    ///
    /// The number of lessons will be equal to one if the couple is the first in the day,
    /// otherwise the list from the change template and the lesson itself will be returned.
    Lessons(Vec<Lesson>),

    /// Street on which the Polytechnic Corps is located.
    Street(String),
}

// noinspection GrazieInspection
/// Obtaining a non-standard type of lesson by name.
fn guess_lesson_type(text: &String) -> Option<LessonType> {
    static MAP: LazyLock<HashMap<&str, LessonType>> = LazyLock::new(|| {
        HashMap::from([
            ("консультация", LessonType::Consultation),
            ("самостоятельная работа", LessonType::IndependentWork),
            ("зачет", LessonType::Exam),
            ("зачет с оценкой", LessonType::ExamWithGrade),
            ("экзамен", LessonType::ExamDefault),
            ("курсовой проект", LessonType::CourseProject),
            ("защита курсового проекта", LessonType::CourseProjectDefense),
        ])
    });

    let name_lower = text.to_lowercase();

    match MAP
        .iter()
        .map(|(text, lesson_type)| (lesson_type, strsim::levenshtein(text, &*name_lower)))
        .filter(|x| x.1 <= 4)
        .min_by_key(|(_, score)| *score)
    {
        None => None,
        Some(v) => Some(v.0.clone()),
    }
}

/// Getting a pair or street from a cell.
fn parse_lesson(
    worksheet: &WorkSheet,
    day: &Day,
    day_boundaries: &Vec<BoundariesCellInfo>,
    lesson_boundaries: &BoundariesCellInfo,
    group_column: u32,
) -> Result<LessonParseResult, ParseError> {
    let row = lesson_boundaries.xls_range.0.0;

    let name = {
        let cell_data = match worksheet.get_string_from_cell(row, group_column) {
            Some(x) => x,
            None => return Ok(Lessons(Vec::new())),
        };

        static OTHER_STREET_RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^[А-Я][а-я]+[,\s]\d+$").unwrap());

        if OTHER_STREET_RE.is_match(&cell_data) {
            return Ok(Street(cell_data));
        }

        cell_data
    };

    let cell_range = worksheet.get_merge_from_start(row, group_column);

    let (default_range, lesson_time) = {
        let end_time_arr = day_boundaries
            .iter()
            .filter(|time| time.xls_range.1.0 == cell_range.1.0)
            .collect::<Vec<&BoundariesCellInfo>>();

        let end_time =
            end_time_arr
                .first()
                .ok_or(ParseError::LessonTimeNotFound(ErrorCellPos {
                    row,
                    column: group_column,
                }))?;

        let range: Option<[u8; 2]> = if lesson_boundaries.default_index != None {
            let default = lesson_boundaries.default_index.unwrap() as u8;
            Some([default, end_time.default_index.unwrap() as u8])
        } else {
            None
        };

        let time = LessonBoundaries {
            start: lesson_boundaries.time_range.start,
            end: end_time.time_range.end,
        };

        Ok((range, time))
    }?;

    let (name, mut subgroups, lesson_type) = parse_name_and_subgroups(&name)?;

    {
        let cabinets: Vec<String> = parse_cabinets(
            worksheet,
            (cell_range.0.0, cell_range.1.0),
            group_column + 1,
        );

        match cabinets.len() {
            // Если кабинетов нет, но есть подгруппы, назначаем им кабинет "??"
            0 => {
                for subgroup in &mut subgroups {
                    subgroup.cabinet = Some("??".to_string());
                }
            }
            // Назначаем этот кабинет всем подгруппам
            1 => {
                for subgroup in &mut subgroups {
                    subgroup.cabinet =
                        Some(cabinets.get(0).or(Some(&String::new())).unwrap().clone())
                }
            }
            len => {
                // Если количество кабинетов совпадает с количеством подгрупп, назначаем кабинеты по порядку
                if len == subgroups.len() {
                    for subgroup in &mut subgroups {
                        subgroup.cabinet = Some(
                            cabinets
                                .get((subgroup.number - 1) as usize)
                                .unwrap()
                                .clone(),
                        );
                    }
                // Если количество кабинетов больше количества подгрупп, делаем ещё одну подгруппу.
                } else if len > subgroups.len() {
                    for index in 0..subgroups.len() {
                        subgroups[index].cabinet = Some(cabinets[index].clone());
                    }

                    while cabinets.len() > subgroups.len() {
                        subgroups.push(LessonSubGroup {
                            number: (subgroups.len() + 1) as u8,
                            cabinet: Some(cabinets[subgroups.len()].clone()),
                            teacher: "Ошибка в расписании".to_string(),
                        });
                    }
                }
            }
        };
    };

    let lesson = Lesson {
        lesson_type: lesson_type.unwrap_or(lesson_boundaries.lesson_type.clone()),
        default_range,
        name: Some(name),
        time: lesson_time,
        subgroups: Some(subgroups),
        group: None,
    };

    let prev_lesson = if day.lessons.is_empty() {
        return Ok(Lessons(Vec::from([lesson])));
    } else {
        &day.lessons[day.lessons.len() - 1]
    };

    Ok(Lessons(Vec::from([
        Lesson {
            lesson_type: Break,
            default_range: None,
            name: None,
            time: LessonBoundaries {
                start: prev_lesson.time.end,
                end: lesson.time.start,
            },
            subgroups: Some(Vec::new()),
            group: None,
        },
        lesson,
    ])))
}

/// Obtaining a list of cabinets to the right of the lesson cell.
fn parse_cabinets(worksheet: &WorkSheet, row_range: (u32, u32), column: u32) -> Vec<String> {
    let mut cabinets: Vec<String> = Vec::new();

    for row in row_range.0..row_range.1 {
        let raw = or_continue!(worksheet.get_string_from_cell(row, column));

        let clean = raw.replace("\n", " ");
        let parts: Vec<&str> = clean.split(" ").collect();

        for part in parts {
            let clean_part = part.to_string().trim().to_string();

            cabinets.push(clean_part);
        }

        break;
    }

    cabinets
}

//noinspection GrazieInspection
/// Getting the "pure" name of the lesson and list of teachers from the text of the lesson cell.
fn parse_name_and_subgroups(
    text: &String,
) -> Result<(String, Vec<LessonSubGroup>, Option<LessonType>), ParseError> {
    // Части названия пары:
    // 1. Само название.
    // 2. Список преподавателей и подгрупп.
    // 3. "Модификатор" (чаще всего).
    //
    // Регулярное выражение для получения ФИО преподавателей и номеров подгрупп (aka. второй части).
    // (?:[А-Я][а-я]+\s?(?:[А-Я][\s.]*){2}(?:\(\d\s?[а-я]+\))?(?:, )?)+[\s.]*
    //
    // Подробнее:
    // (?:
    //     [А-Я][а-я]+         - Фамилия.
    //     \s?                 - Кто знает, будет ли там пробел.
    //     (?:[А-Я][\s.]*){2}  - Имя и отчество с учётом случайных пробелов и точек.
    //     (?:
    //         \(              - Открытие подгруппы.
    //         \s?             - Кто знает, будет ли там пробел.
    //         \d              - Номер подгруппы.
    //         \s?             - Кто знает, будет ли там пробел.
    //         [а-я\s]+        - Слово "подгруппа" с учётов ошибок.
    //         \)              - Закрытие подгруппы.
    //     )?                  - Явное указание подгруппы может отсутствовать по понятным причинам.
    //     (?:, )?             - Разделители между отдельными частями.
    // )+
    // [\s.]*                  - Забираем с собой всякий мусор, что бы не передать его в третью часть.

    static NAMES_REGEX: LazyLock<Regex> = LazyLock::new(|| {
        Regex::new(
            r"(?:[А-Я][а-я]+\s?(?:[А-Я][\s.]*){2}(?:\(\s*\d\s*[а-я\s]+\))?(?:[\s,]+)?)+[\s.]*",
        )
        .unwrap()
    });

    // Отчистка
    static CLEAN_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[\s\n\t]+").unwrap());

    let text = CLEAN_RE
        .replace(&text.replace(&[' ', '\t', '\n'], " "), " ")
        .to_string();

    let (lesson_name, mut subgroups, lesson_type) = match NAMES_REGEX.captures(&text) {
        Some(captures) => {
            let capture = captures.get(0).unwrap();

            let subgroups: Vec<LessonSubGroup> = {
                let src = capture.as_str().replace(&[' ', '.'], "");

                src.split(',')
                    .map(|name| {
                        let open_bracket_index = name.find('(');

                        let subgroup_number = open_bracket_index.map_or(0, |index| {
                            name[(index + 1)..(index + 2)].parse::<u8>().unwrap()
                        });

                        let teacher_name = {
                            let name_end = open_bracket_index.unwrap_or_else(|| name.len());

                            // Я ебал. Как же я долго до этого доходил.
                            format!(
                                "{} {}.{}.",
                                name.get(..name_end - 4).unwrap(),
                                name.get(name_end - 4..name_end - 2).unwrap(),
                                name.get(name_end - 2..name_end).unwrap(),
                            )
                        };

                        LessonSubGroup {
                            number: subgroup_number,
                            cabinet: None,
                            teacher: teacher_name,
                        }
                    })
                    .collect()
            };

            let name = text[..capture.start()].trim().to_string();
            let extra = text[capture.end()..].trim().to_string();

            let lesson_type = if extra.len() > 4 {
                let result = guess_lesson_type(&extra);

                #[cfg(not(debug_assertions))]
                if result.is_none() {
                    sentry::capture_message(
                        &*format!("Не удалось угадать тип пары '{}'!", extra),
                        sentry::Level::Warning,
                    );
                }

                result
            } else {
                None
            };

            (name, subgroups, lesson_type)
        }
        None => (text, Vec::new(), None),
    };

    // фикс, если у кого-то отсутствует индекс подгруппы

    if subgroups.len() == 1 {
        let index = subgroups[0].number;

        if index == 0 {
            subgroups[0].number = 1u8;
        } else {
            subgroups.push(LessonSubGroup {
                number: if index == 1 { 2 } else { 1 },
                cabinet: None,
                teacher: "Только у другой".to_string(),
            });
        }
    } else if subgroups.len() == 2 {
        // если индексы отсутствуют у обоих, ставим поочерёдно
        if subgroups[0].number == 0 && subgroups[1].number == 0 {
            subgroups[0].number = 1;
            subgroups[1].number = 2;
        }
        // если индекс отсутствует у первого, ставим 2, если у второго индекс 1 и наоборот
        else if subgroups[0].number == 0 {
            subgroups[0].number = if subgroups[1].number == 1 { 2 } else { 1 };
        }
        // если индекс отсутствует у второго, ставим 2, если у первого индекс 1 и наоборот
        else if subgroups[1].number == 0 {
            subgroups[1].number = if subgroups[0].number == 1 { 2 } else { 1 };
        }
    }

    if subgroups.len() == 2 && subgroups[0].number == 2 && subgroups[1].number == 1 {
        subgroups.reverse()
    }

    Ok((lesson_name, subgroups, lesson_type))
}

/// Getting the start and end of a pair from a cell in the first column of a document.
///
/// # Arguments
///
/// * `cell_data`: text in cell.
/// * `date`: date of the current day.
fn parse_lesson_boundaries_cell(
    cell_data: &String,
    date: DateTime<Utc>,
) -> Option<LessonBoundaries> {
    static TIME_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(\d+\.\d+)-(\d+\.\d+)").unwrap());

    let parse_res = if let Some(captures) = TIME_RE.captures(cell_data) {
        captures
    } else {
        return None;
    };

    let start_match = parse_res.get(1).unwrap().as_str();
    let start_parts: Vec<&str> = start_match.split(".").collect();

    let end_match = parse_res.get(2).unwrap().as_str();
    let end_parts: Vec<&str> = end_match.split(".").collect();

    static GET_TIME: fn(DateTime<Utc>, &Vec<&str>) -> DateTime<Utc> = |date, parts| {
        date + Duration::hours(parts[0].parse::<i64>().unwrap() - 4)
            + Duration::minutes(parts[1].parse::<i64>().unwrap())
    };

    Some(LessonBoundaries {
        start: GET_TIME(date.clone(), &start_parts),
        end: GET_TIME(date, &end_parts),
    })
}

/// Parse the column of the document to obtain a list of day's lesson boundaries.
///
/// # Arguments
///
/// * `worksheet`: document.
/// * `date`: date of the current day.
/// * `row_range`: row boundaries of the current day.
/// * `column`: column with the required data.
fn parse_day_boundaries(
    worksheet: &WorkSheet,
    date: DateTime<Utc>,
    row_range: (u32, u32),
    column: u32,
) -> Result<Vec<BoundariesCellInfo>, ParseError> {
    let mut day_times: Vec<BoundariesCellInfo> = Vec::new();

    for row in row_range.0..row_range.1 {
        let time_cell = if let Some(str) = worksheet.get_string_from_cell(row, column) {
            str
        } else {
            continue;
        };

        let lesson_time = parse_lesson_boundaries_cell(&time_cell, date.clone()).ok_or(
            ParseError::LessonBoundaries(ErrorCell::new(row, column, time_cell.clone())),
        )?;

        // type
        let lesson_type = if time_cell.contains("пара") {
            LessonType::Default
        } else {
            LessonType::Additional
        };

        // lesson index
        let default_index = if lesson_type == LessonType::Default {
            Some(
                time_cell
                    .chars()
                    .next()
                    .unwrap()
                    .to_string()
                    .parse::<u32>()
                    .unwrap(),
            )
        } else {
            None
        };

        day_times.push(BoundariesCellInfo {
            time_range: lesson_time,
            lesson_type,
            default_index,
            xls_range: worksheet.get_merge_from_start(row, column),
        });
    }

    Ok(day_times)
}

/// Parse the column of the document to obtain a list of week's lesson boundaries.
///
/// # Arguments
///
/// * `worksheet`: document.
/// * `week_markup`: markup of the current week.
fn parse_week_boundaries(
    worksheet: &WorkSheet,
    week_markup: &Vec<DayCellInfo>,
) -> Result<Vec<Vec<BoundariesCellInfo>>, ParseError> {
    let mut result: Vec<Vec<BoundariesCellInfo>> = Vec::new();

    let worksheet_end_row = worksheet.end().unwrap().0;
    let lesson_time_column = week_markup[0].column + 1;

    for day_index in 0..week_markup.len() {
        let day_markup = &week_markup[day_index];

        // Если текущий день не последнему, то индекс строки следующего дня.
        // Если текущий день - последний, то индекс последней строки документа.
        let end_row = if day_index != week_markup.len() - 1 {
            week_markup[day_index + 1].row
        } else {
            worksheet_end_row
        };

        let day_boundaries = parse_day_boundaries(
            &worksheet,
            day_markup.date.clone(),
            (day_markup.row, end_row),
            lesson_time_column,
        )?;

        result.push(day_boundaries);
    }

    Ok(result)
}

/// Conversion of the list of couples of groups in the list of lessons of teachers.
fn convert_groups_to_teachers(
    groups: &HashMap<String, ScheduleEntry>,
) -> HashMap<String, ScheduleEntry> {
    let mut teachers: HashMap<String, ScheduleEntry> = HashMap::new();

    let empty_days: Vec<Day> = groups
        .values()
        .next()
        .unwrap()
        .days
        .iter()
        .map(|day| Day {
            name: day.name.clone(),
            street: day.street.clone(),
            date: day.date.clone(),
            lessons: vec![],
        })
        .collect();

    for group in groups.values() {
        for (index, day) in group.days.iter().enumerate() {
            for group_lesson in &day.lessons {
                if group_lesson.lesson_type == Break {
                    continue;
                }

                if group_lesson.subgroups.is_none() {
                    continue;
                }

                let subgroups = group_lesson.subgroups.as_ref().unwrap();

                for subgroup in subgroups {
                    if subgroup.teacher == "Ошибка в расписании" {
                        continue;
                    }

                    if !teachers.contains_key(&subgroup.teacher) {
                        teachers.insert(
                            subgroup.teacher.clone(),
                            ScheduleEntry {
                                name: subgroup.teacher.clone(),
                                days: empty_days.to_vec(),
                            },
                        );
                    }

                    let teacher_day = teachers
                        .get_mut(&subgroup.teacher)
                        .unwrap()
                        .days
                        .get_mut(index)
                        .unwrap();

                    teacher_day.lessons.push({
                        let mut lesson = group_lesson.clone();
                        lesson.group = Some(group.name.clone());

                        lesson
                    });
                }
            }
        }
    }

    teachers.iter_mut().for_each(|(_, teacher)| {
        teacher.days.iter_mut().for_each(|day| {
            day.lessons.sort_by(|a, b| {
                a.default_range.as_ref().unwrap()[1].cmp(&b.default_range.as_ref().unwrap()[1])
            })
        })
    });

    teachers
}

/// Reading XLS Document from the buffer and converting it into the schedule ready to use.
///
/// # Arguments
///
/// * `buffer`: XLS data containing schedule.
///
/// returns: Result<ParseResult, ParseError>
///
/// # Examples
///
/// ```
/// use schedule_parser::parse_xls;
///
/// let result = parse_xls(&include_bytes!("../../schedule.xls").to_vec());
///
/// assert!(result.is_ok(), "{}", result.err().unwrap());
///
/// assert_ne!(result.as_ref().unwrap().groups.len(), 0);
/// assert_ne!(result.as_ref().unwrap().teachers.len(), 0);
/// ```
pub fn parse_xls(buffer: &Vec<u8>) -> Result<ParseResult, ParseError> {
    let cursor = Cursor::new(&buffer);
    let mut workbook: Xls<_> =
        open_workbook_from_rs(cursor).map_err(|e| ParseError::BadXLS(std::sync::Arc::new(e)))?;

    let worksheet = {
        let (worksheet_name, worksheet) = workbook
            .worksheets()
            .first()
            .ok_or(ParseError::NoWorkSheets)?
            .clone();

        let worksheet_merges = workbook
            .worksheet_merge_cells(&*worksheet_name)
            .ok_or(ParseError::NoWorkSheets)?;

        WorkSheet {
            data: worksheet,
            merges: worksheet_merges,
        }
    };

    let (week_markup, groups_markup) = parse_skeleton(&worksheet)?;
    let week_boundaries = parse_week_boundaries(&worksheet, &week_markup)?;

    let mut groups: HashMap<String, ScheduleEntry> = HashMap::new();

    for group_markup in groups_markup {
        let mut group = ScheduleEntry {
            name: group_markup.name,
            days: Vec::new(),
        };

        for day_index in 0..(&week_markup).len() {
            let day_markup = &week_markup[day_index];

            let mut day = Day {
                name: day_markup.name.clone(),
                street: None,
                date: day_markup.date,
                lessons: Vec::new(),
            };

            let day_boundaries = &week_boundaries[day_index];

            for lesson_boundaries in day_boundaries {
                match &mut parse_lesson(
                    &worksheet,
                    &day,
                    &day_boundaries,
                    &lesson_boundaries,
                    group_markup.column,
                )? {
                    Lessons(lesson) => day.lessons.append(lesson),
                    Street(street) => day.street = Some(street.to_owned()),
                }
            }

            group.days.push(day);
        }

        groups.insert(group.name.clone(), group);
    }

    Ok(ParseResult {
        teachers: convert_groups_to_teachers(&groups),
        groups,
    })
}

#[cfg(any(test, feature = "test-utils"))]
pub mod test_utils {
    use super::*;

    pub fn test_result() -> Result<ParseResult, ParseError> {
        parse_xls(&include_bytes!("../../schedule.xls").to_vec())
    }
}

#[cfg(test)]
pub mod tests {
    #[test]
    fn read() {
        let result = super::test_utils::test_result();

        assert!(result.is_ok(), "{}", result.err().unwrap());

        assert_ne!(result.as_ref().unwrap().groups.len(), 0);
        assert_ne!(result.as_ref().unwrap().teachers.len(), 0);
    }

    #[test]
    fn test_split_lesson() {
        let result = super::test_utils::test_result();
        assert!(result.is_ok(), "{}", result.err().unwrap());

        let result = result.unwrap();
        assert!(result.groups.contains_key("ИС-214/23"));

        let group = result.groups.get("ИС-214/23").unwrap();

        let thursday = group.days.get(3).unwrap();
        assert_eq!(thursday.lessons.len(), 1);

        let lesson = &thursday.lessons[0];
        assert_eq!(lesson.default_range.unwrap()[1], 3);
        assert!(lesson.subgroups.is_some());

        let subgroups = lesson.subgroups.as_ref().unwrap();
        assert_eq!(subgroups.len(), 2);
        assert_eq!(subgroups[0].cabinet, Some("44".to_string()));
        assert_eq!(subgroups[1].cabinet, Some("43".to_string()));
    }
}
