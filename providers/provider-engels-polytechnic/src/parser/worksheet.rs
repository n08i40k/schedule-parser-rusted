use regex::Regex;
use std::fmt::{Display, Formatter};
use std::ops::Deref;
use std::sync::LazyLock;

/// XLS WorkSheet data.
pub struct WorkSheet {
    pub data: calamine::Range<calamine::Data>,
    pub merges: Vec<calamine::Dimensions>,
}

#[derive(Clone, Debug, derive_more::Error)]
pub struct CellPos {
    pub row: u32,
    pub column: u32,
}

fn format_column_index(index: u32) -> String {
    // https://stackoverflow.com/a/297214
    let quotient = index / 26;

    let char = char::from((65 + (index % 26)) as u8);

    if quotient > 0 {
        return format!("{}{}", format_column_index(quotient - 1), char);
    }

    char.to_string()
}

impl Display for CellPos {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_fmt(format_args!(
            "column {}, row {}",
            format_column_index(self.column),
            self.row + 1,
        ))
    }
}

pub struct CellRange {
    pub start: CellPos,
    pub end: CellPos,
}

impl Deref for WorkSheet {
    type Target = calamine::Range<calamine::Data>;

    fn deref(&self) -> &Self::Target {
        &self.data
    }
}

impl WorkSheet {
    /// Getting a line from the required cell.
    pub fn get_string_from_cell(&self, row: u32, col: u32) -> Option<String> {
        let cell_data = if let Some(data) = self.get((row as usize, col as usize)) {
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
    pub fn get_merge_from_start(&self, row: u32, column: u32) -> CellRange {
        match self
            .merges
            .iter()
            .find(|merge| merge.start.0 == row && merge.start.1 == column)
        {
            Some(merge) => CellRange {
                start: CellPos::new(merge.start.0, merge.start.1),
                end: CellPos::new(merge.end.0 + 1, merge.end.1 + 1),
            },
            None => CellRange {
                start: CellPos::new(row, column),
                end: CellPos::new(row + 1, column + 1),
            },
        }
    }
}

impl CellPos {
    pub fn new(row: u32, column: u32) -> Self {
        Self { row, column }
    }
}
