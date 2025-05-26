use crate::LessonParseResult::{Lessons, Street};
use crate::schema::LessonType::Break;
use crate::schema::{
    Day, ErrorCell, ErrorCellPos, Lesson, LessonBoundaries, LessonSubGroup, LessonType, ParseError,
    ParseResult, ScheduleEntry,
};
use calamine::{Reader, Xls, open_workbook_from_rs};
use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use regex::Regex;
use std::collections::HashMap;
use std::io::Cursor;
use std::ops::Deref;
use std::sync::LazyLock;

mod macros;
pub mod schema;

/// Data cell storing the group name.
struct GroupCellInfo {
    /// Column index.
    column: u32,

    /// Text in the cell.
    name: String,
}

/// Data cell storing the line.
struct DayCellInfo {
    /// Line index.
    row: u32,

    /// Column index.
    column: u32,

    /// Day name.
    name: String,

    /// Date of the day.
    date: DateTime<Utc>,
}

/// Data on the time of lessons from the second column of the schedule.
struct BoundariesCellInfo {
    /// Temporary segment of the lesson.
    time_range: LessonBoundaries,

    /// Type of lesson.
    lesson_type: LessonType,

    /// The lesson index.
    default_index: Option<u32>,

    /// The frame of the cell.
    xls_range: ((u32, u32), (u32, u32)),
}

/// Working sheet type alias.
type WorkSheet = calamine::Range<calamine::Data>;

/// Getting a line from the required cell.
fn get_string_from_cell(worksheet: &WorkSheet, row: u32, col: u32) -> Option<String> {
    let cell_data = if let Some(data) = worksheet.get((row as usize, col as usize)) {
        data.to_string()
    } else {
        return None;
    };

    if cell_data.trim().is_empty() {
        return None;
    }

    static NL_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[\n\r]+").unwrap());
    static SP_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\s+").unwrap());

    let trimmed_data = SP_RE
        .replace_all(&NL_RE.replace_all(&cell_data, " "), " ")
        .trim()
        .to_string();

    if trimmed_data.is_empty() {
        None
    } else {
        Some(trimmed_data)
    }
}

/// Obtaining the boundaries of the cell along its upper left coordinate.
fn get_merge_from_start(worksheet: &WorkSheet, row: u32, column: u32) -> ((u32, u32), (u32, u32)) {
    let worksheet_end = worksheet.end().unwrap();

    let row_end: u32 = {
        let mut r: u32 = 0;

        for _r in (row + 1)..worksheet_end.0 {
            r = _r;

            if let Some(_) = worksheet.get((_r as usize, column as usize)) {
                break;
            }
        }

        r
    };

    let column_end: u32 = {
        let mut c: u32 = 0;

        for _c in (column + 1)..worksheet_end.1 {
            c = _c;

            if let Some(_) = worksheet.get((row as usize, _c as usize)) {
                break;
            }
        }

        c
    };

    ((row, column), (row_end, column_end))
}

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

        let day_full_name = or_continue!(get_string_from_cell(&worksheet, row, 0));

        // parse groups row when days column will found
        if groups.is_empty() {
            // переход на предыдущую строку
            row -= 1;

            for column in (worksheet_start.1 + 2)..=worksheet_end.1 {
                groups.push(GroupCellInfo {
                    column,
                    name: or_continue!(get_string_from_cell(&worksheet, row, column)),
                });
            }

            // возврат на текущую строку
            row += 1;
        }

        let (day_name, day_date) = {
            let space_index = day_full_name.find(' ').unwrap();

            let name = day_full_name[..space_index].to_string();

            let date_raw = day_full_name[space_index + 1..].to_string();
            let date_add = format!("{} 00:00:00", date_raw);

            let date =
                or_break!(NaiveDateTime::parse_from_str(&*date_add, "%d.%m.%Y %H:%M:%S").ok());

            (name, date.and_utc())
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

trait StringInnerSlice {
    /// Obtaining a line from the line on the initial and final index.
    fn inner_slice(&self, from: usize, to: usize) -> Self;
}

impl StringInnerSlice for String {
    fn inner_slice(&self, from: usize, to: usize) -> Self {
        self.chars()
            .take(from)
            .chain(self.chars().skip(to))
            .collect()
    }
}

// noinspection GrazieInspection
/// Obtaining a non-standard type of lesson by name.
fn guess_lesson_type(name: &String) -> Option<(String, LessonType)> {
    let map: HashMap<String, LessonType> = HashMap::from([
        ("(консультация)".to_string(), LessonType::Consultation),
        (
            "самостоятельная работа".to_string(),
            LessonType::IndependentWork,
        ),
        ("зачет".to_string(), LessonType::Exam),
        ("зачет с оценкой".to_string(), LessonType::ExamWithGrade),
        ("экзамен".to_string(), LessonType::ExamDefault),
    ]);

    let matcher = SkimMatcherV2::default();
    let name_lower = name.to_lowercase();

    type SearchResult<'a> = (&'a LessonType, i64, Vec<usize>);

    let mut search_results: Vec<SearchResult> = map
        .iter()
        .map(|entry| -> SearchResult {
            if let Some((score, indices)) = matcher.fuzzy_indices(&*name_lower, entry.0) {
                return (entry.1, score, indices);
            }

            (entry.1, 0, Vec::new())
        })
        .collect();
    search_results.sort_by(|a, b| b.1.cmp(&a.1));

    let guessed_type = search_results.first().unwrap();

    if guessed_type.1 > 80 {
        Some((
            name.inner_slice(guessed_type.2[0], guessed_type.2[guessed_type.2.len() - 1]),
            guessed_type.0.clone(),
        ))
    } else {
        None
    }
}

/// Getting a pair or street from a cell.
fn parse_lesson(
    worksheet: &WorkSheet,
    day: &mut Day,
    day_boundaries: &Vec<BoundariesCellInfo>,
    lesson_boundaries: &BoundariesCellInfo,
    column: u32,
) -> Result<LessonParseResult, ParseError> {
    let row = lesson_boundaries.xls_range.0.0;

    let (name, lesson_type) = {
        let full_name = match get_string_from_cell(&worksheet, row, column) {
            Some(x) => x,
            None => return Ok(Lessons(Vec::new())),
        };

        static OTHER_STREET_RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^[А-Я][а-я]+,?\s?[0-9]+$").unwrap());

        if OTHER_STREET_RE.is_match(&full_name) {
            return Ok(Street(full_name));
        }

        match guess_lesson_type(&full_name) {
            Some(x) => x,
            None => (full_name, lesson_boundaries.lesson_type.clone()),
        }
    };

    let (default_range, lesson_time) = {
        let cell_range = get_merge_from_start(worksheet, row, column);
        
        let end_time_arr = day_boundaries
            .iter()
            .filter(|time| time.xls_range.1.0 == cell_range.1.0)
            .collect::<Vec<&BoundariesCellInfo>>();

        let end_time = end_time_arr
            .first()
            .ok_or(ParseError::LessonTimeNotFound(ErrorCellPos { row, column }))?;

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

    let (name, mut subgroups) = parse_name_and_subgroups(&name)?;

    {
        let cabinets: Vec<String> = parse_cabinets(worksheet, row, column + 1);

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
        lesson_type,
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
fn parse_cabinets(worksheet: &WorkSheet, row: u32, column: u32) -> Vec<String> {
    let mut cabinets: Vec<String> = Vec::new();

    if let Some(raw) = get_string_from_cell(&worksheet, row, column) {
        let clean = raw.replace("\n", " ");
        let parts: Vec<&str> = clean.split(" ").collect();

        for part in parts {
            let clean_part = part.to_string().trim().to_string();

            cabinets.push(clean_part);
        }
    }

    cabinets
}

/// Getting the "pure" name of the lesson and list of teachers from the text of the lesson cell.
fn parse_name_and_subgroups(name: &String) -> Result<(String, Vec<LessonSubGroup>), ParseError> {
    static LESSON_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"(?:[А-Я][а-я]+[А-Я]{2}(?:\([0-9][а-я]+\))?)+$").unwrap());
    static TEACHER_RE: LazyLock<Regex> =
        LazyLock::new(|| Regex::new(r"([А-Я][а-я]+)([А-Я])([А-Я])(?:\(([0-9])[а-я]+\))?").unwrap());
    static CLEAN_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[\s.,]+").unwrap());
    static END_CLEAN_RE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[.\s]+$").unwrap());

    let (teachers, lesson_name) = {
        let clean_name = CLEAN_RE.replace_all(&name, "").to_string();

        if let Some(captures) = LESSON_RE.captures(&clean_name) {
            let capture = captures.get(0).unwrap();
            let capture_str = capture.as_str().to_string();
            let capture_name: String = capture_str.chars().take(5).collect();

            (
                END_CLEAN_RE.replace(&capture_str, "").to_string(),
                END_CLEAN_RE
                    .replace(&name[0..name.find(&*capture_name).unwrap()], "")
                    .to_string(),
            )
        } else {
            return Ok((END_CLEAN_RE.replace(&name, "").to_string(), Vec::new()));
        }
    };

    let mut subgroups: Vec<LessonSubGroup> = Vec::new();

    let teacher_it = TEACHER_RE.captures_iter(&teachers);

    for captures in teacher_it {
        subgroups.push(LessonSubGroup {
            number: match captures.get(4) {
                Some(capture) => capture.as_str().to_string().parse::<u8>().unwrap(),
                None => 0,
            },
            cabinet: None,
            teacher: format!(
                "{} {}.{}.",
                captures.get(1).unwrap().as_str().to_string(),
                captures.get(2).unwrap().as_str().to_string(),
                captures.get(3).unwrap().as_str().to_string()
            ),
        });
    }

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

    Ok((lesson_name, subgroups))
}

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

fn parse_day_boundaries_column(
    worksheet: &WorkSheet,
    day_markup: &DayCellInfo,
    lesson_time_column: u32,
    row_distance: u32,
) -> Result<Vec<BoundariesCellInfo>, ParseError> {
    let mut day_times: Vec<BoundariesCellInfo> = Vec::new();

    for row in day_markup.row..(day_markup.row + row_distance) {
        let time_cell = if let Some(str) = get_string_from_cell(&worksheet, row, lesson_time_column)
        {
            str
        } else {
            continue;
        };

        let lesson_time = parse_lesson_boundaries_cell(&time_cell, day_markup.date.clone()).ok_or(
            ParseError::LessonBoundaries(ErrorCell::new(
                row,
                lesson_time_column,
                time_cell.clone(),
            )),
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
            xls_range: get_merge_from_start(&worksheet, row, lesson_time_column),
        });
    }

    return Ok(day_times);
}

fn parse_week_boundaries_column(
    worksheet: &WorkSheet,
    week_markup: &Vec<DayCellInfo>,
) -> Result<Vec<Vec<BoundariesCellInfo>>, ParseError> {
    let mut result: Vec<Vec<BoundariesCellInfo>> = Vec::new();

    let worksheet_end_row = worksheet.end().unwrap().0;
    let lesson_time_column = week_markup[0].column + 1;

    for day_index in 0..week_markup.len() {
        let day_markup = &week_markup[day_index];

        // Если текущий день не последнему, то индекс строки следующего дня минус индекс строки текущего дня.
        // Если текущий день - последний, то индекс последней строки документа минус индекс строки текущего дня.
        let row_distance = if day_index != week_markup.len() - 1 {
            week_markup[day_index + 1].row
        } else {
            worksheet_end_row
        } - day_markup.row;

        let day_boundaries =
            parse_day_boundaries_column(&worksheet, day_markup, lesson_time_column, row_distance)?;

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

    let worksheet: WorkSheet = workbook
        .worksheets()
        .first()
        .ok_or(ParseError::NoWorkSheets)?
        .1
        .to_owned();

    let (week_markup, groups_markup) = parse_skeleton(&worksheet)?;
    let week_boundaries = parse_week_boundaries_column(&worksheet, &week_markup)?;

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
                    &mut day,
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
        assert_eq!(thursday.lessons[0].default_range.unwrap()[1], 3);
    }
}
