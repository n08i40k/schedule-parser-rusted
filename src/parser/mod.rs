use crate::parser::LessonParseResult::{Lessons, Street};
use crate::parser::schema::LessonType::Break;
use crate::parser::schema::{
    Day, ErrorCell, ErrorCellPos, Lesson, LessonSubGroup, LessonTime, LessonType, ParseError,
    ParseResult, ScheduleEntry,
};
use calamine::{Reader, Xls, open_workbook_from_rs};
use chrono::{DateTime, Duration, NaiveDateTime, Utc};
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;
use regex::Regex;
use std::collections::HashMap;
use std::io::Cursor;
use std::sync::LazyLock;

pub mod schema;

/// Data cell storing the line.
struct InternalId {
    /// Line index.
    row: u32,

    /// Column index.
    column: u32,

    /// Text in the cell.
    name: String,
}

/// Data on the time of lessons from the second column of the schedule.
struct InternalTime {
    /// Temporary segment of the lesson.
    time_range: LessonTime,

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
fn parse_skeleton(worksheet: &WorkSheet) -> Result<(Vec<InternalId>, Vec<InternalId>), ParseError> {
    let range = &worksheet;

    let mut is_parsed = false;

    let mut groups: Vec<InternalId> = Vec::new();
    let mut days: Vec<InternalId> = Vec::new();

    let start = range.start().ok_or(ParseError::UnknownWorkSheetRange)?;
    let end = range.end().ok_or(ParseError::UnknownWorkSheetRange)?;

    let mut row = start.0;
    while row < end.0 {
        row += 1;

        let day_name_opt = get_string_from_cell(&worksheet, row, 0);
        if day_name_opt.is_none() {
            continue;
        }

        let day_name = day_name_opt.unwrap();

        if !is_parsed {
            is_parsed = true;

            row -= 1;

            for column in (start.1 + 2)..=end.1 {
                let group_name = get_string_from_cell(&worksheet, row, column);
                if group_name.is_none() {
                    continue;
                }

                groups.push(InternalId {
                    row,
                    column,
                    name: group_name.unwrap(),
                });
            }

            row += 1;
        }

        days.push(InternalId {
            row,
            column: 0,
            name: day_name.clone(),
        });

        if days.len() > 2 && day_name.starts_with("Суббота") {
            break;
        }
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
    day_times: &Vec<InternalTime>,
    time: &InternalTime,
    column: u32,
) -> Result<LessonParseResult, ParseError> {
    let row = time.xls_range.0.0;

    let (name, lesson_type) = {
        let raw_name_opt = get_string_from_cell(&worksheet, row, column);
        if raw_name_opt.is_none() {
            return Ok(Lessons(Vec::new()));
        }

        let raw_name = raw_name_opt.unwrap();

        static OTHER_STREET_RE: LazyLock<Regex> =
            LazyLock::new(|| Regex::new(r"^[А-Я][а-я]+,?\s?[0-9]+$").unwrap());

        if OTHER_STREET_RE.is_match(&raw_name) {
            return Ok(Street(raw_name));
        }

        if let Some(guess) = guess_lesson_type(&raw_name) {
            guess
        } else {
            (raw_name, time.lesson_type.clone())
        }
    };

    let (default_range, lesson_time) = || -> Result<(Option<[u8; 2]>, LessonTime), ParseError> {
        // check if multi-lesson
        let cell_range = get_merge_from_start(worksheet, row, column);

        let end_time_arr = day_times
            .iter()
            .filter(|time| time.xls_range.1.0 == cell_range.1.0)
            .collect::<Vec<&InternalTime>>();

        let end_time = end_time_arr
            .first()
            .ok_or(ParseError::LessonTimeNotFound(ErrorCellPos { row, column }))?;

        let range: Option<[u8; 2]> = if time.default_index != None {
            let default = time.default_index.unwrap() as u8;
            Some([default, end_time.default_index.unwrap() as u8])
        } else {
            None
        };

        let time = LessonTime {
            start: time.time_range.start,
            end: end_time.time_range.end,
        };

        Ok((range, time))
    }()?;

    let (name, mut subgroups) = parse_name_and_subgroups(&name)?;

    {
        let cabinets: Vec<String> = parse_cabinets(worksheet, row, column + 1);

        // Если количество кабинетов равно 1, назначаем этот кабинет всем подгруппам
        if cabinets.len() == 1 {
            for subgroup in &mut subgroups {
                subgroup.cabinet = Some(cabinets.get(0).or(Some(&String::new())).unwrap().clone())
            }
        }
        // Если количество кабинетов совпадает с количеством подгрупп, назначаем кабинеты по порядку
        else if cabinets.len() == subgroups.len() {
            for subgroup in &mut subgroups {
                subgroup.cabinet = Some(
                    cabinets
                        .get((subgroup.number - 1) as usize)
                        .unwrap()
                        .clone(),
                );
            }
        }
        // Если количество кабинетов больше количества подгрупп, делаем ещё одну подгруппу.
        else if cabinets.len() > subgroups.len() {
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
        // Если кабинетов нет, но есть подгруппы, назначаем им значение "??"
        else {
            for subgroup in &mut subgroups {
                subgroup.cabinet = Some("??".to_string());
            }
        }

        cabinets
    };

    let lesson = Lesson {
        lesson_type,
        default_range,
        name: Some(name),
        time: lesson_time,
        subgroups: Some(subgroups),
        group: None,
    };

    let prev_lesson = if day.lessons.len() == 0 {
        return Ok(Lessons(Vec::from([lesson])));
    } else {
        &day.lessons[day.lessons.len() - 1]
    };

    Ok(Lessons(Vec::from([
        Lesson {
            lesson_type: Break,
            default_range: None,
            name: None,
            time: LessonTime {
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
/// use schedule_parser_rusted::parser::parse_xls;
///
/// let result = parse_xls(&include_bytes!("../../schedule.xls").to_vec());
///
/// assert!(result.is_ok());
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

    let (days_markup, groups_markup) = parse_skeleton(&worksheet)?;

    let mut groups: HashMap<String, ScheduleEntry> = HashMap::new();
    let mut days_times: Vec<Vec<InternalTime>> = Vec::new();

    let saturday_end_row = worksheet.end().unwrap().0;

    for group_markup in groups_markup {
        let mut group = ScheduleEntry {
            name: group_markup.name,
            days: Vec::new(),
        };

        for day_index in 0..(&days_markup).len() {
            let day_markup = &days_markup[day_index];

            let mut day = {
                let space_index = day_markup.name.find(' ').unwrap();

                let name = day_markup.name[..space_index].to_string();

                let date_raw = day_markup.name[space_index + 1..].to_string();
                let date_add = format!("{} 00:00:00", date_raw);

                let date = NaiveDateTime::parse_from_str(&*date_add, "%d.%m.%Y %H:%M:%S");

                Day {
                    name,
                    street: None,
                    date: date.unwrap().and_utc(),
                    lessons: Vec::new(),
                }
            };

            let lesson_time_column = days_markup[0].column + 1;

            let row_distance = if day_index != days_markup.len() - 1 {
                days_markup[day_index + 1].row
            } else {
                saturday_end_row
            } - day_markup.row;

            if days_times.len() != 6 {
                let mut day_times: Vec<InternalTime> = Vec::new();

                for row in day_markup.row..(day_markup.row + row_distance) {
                    // time
                    let time_opt = get_string_from_cell(&worksheet, row, lesson_time_column);
                    if time_opt.is_none() {
                        continue;
                    }

                    let time = time_opt.unwrap();

                    // type
                    let lesson_type = if time.contains("пара") {
                        LessonType::Default
                    } else {
                        LessonType::Additional
                    };

                    // lesson index
                    let default_index = if lesson_type == LessonType::Default {
                        Some(
                            time.chars()
                                .next()
                                .unwrap()
                                .to_string()
                                .parse::<u32>()
                                .unwrap(),
                        )
                    } else {
                        None
                    };

                    // time
                    let time_range = {
                        static TIME_RE: LazyLock<Regex> =
                            LazyLock::new(|| Regex::new(r"(\d+\.\d+)-(\d+\.\d+)").unwrap());

                        let parse_res = TIME_RE.captures(&time).ok_or(ParseError::GlobalTime(
                            ErrorCell::new(row, lesson_time_column, time.clone()),
                        ))?;

                        let start_match = parse_res.get(1).unwrap().as_str();
                        let start_parts: Vec<&str> = start_match.split(".").collect();

                        let end_match = parse_res.get(2).unwrap().as_str();
                        let end_parts: Vec<&str> = end_match.split(".").collect();

                        static GET_TIME: fn(DateTime<Utc>, &Vec<&str>) -> DateTime<Utc> =
                            |date, parts| {
                                date + Duration::hours(parts[0].parse::<i64>().unwrap() - 4)
                                    + Duration::minutes(parts[1].parse::<i64>().unwrap())
                            };

                        LessonTime {
                            start: GET_TIME(day.date.clone(), &start_parts),
                            end: GET_TIME(day.date.clone(), &end_parts),
                        }
                    };

                    day_times.push(InternalTime {
                        time_range,
                        lesson_type,
                        default_index,
                        xls_range: get_merge_from_start(&worksheet, row, lesson_time_column),
                    });
                }

                days_times.push(day_times);
            }

            let day_times = &days_times[day_index];

            for time in day_times {
                match &mut parse_lesson(
                    &worksheet,
                    &mut day,
                    &day_times,
                    &time,
                    group_markup.column,
                )? {
                    Lessons(l) => day.lessons.append(l),
                    Street(s) => day.street = Some(s.to_owned()),
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

#[cfg(test)]
pub mod tests {
    use super::*;

    pub fn test_result() -> Result<ParseResult, ParseError> {
        parse_xls(&include_bytes!("../../schedule.xls").to_vec())
    }

    #[test]
    fn read() {
        let result = test_result();

        assert!(result.is_ok());

        assert_ne!(result.as_ref().unwrap().groups.len(), 0);
        assert_ne!(result.as_ref().unwrap().teachers.len(), 0);
    }
}
