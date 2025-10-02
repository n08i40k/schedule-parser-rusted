use derive_more::{Display, Error, From};
use crate::parser::worksheet::CellPos;

#[derive(Clone, Debug, Display, Error)]
#[display("'{data}' at {pos}")]
pub struct ErrorCell {
    pub pos: CellPos,
    pub data: String,
}

impl ErrorCell {
    pub fn new(row: u32, column: u32, data: &str) -> Self {
        Self {
            pos: CellPos { row, column },
            data: data.to_string(),
        }
    }
}

#[derive(Debug, Display, Error, From)]
pub enum Error {
    #[from]
    BadXls(calamine::XlsError),

    #[display("No work sheets found.")]
    NoWorkSheets,

    #[display("There is no data on work sheet boundaries.")]
    UnknownWorkSheetRange,

    #[display("Failed to read lesson start and end from {_0}.")]
    NoLessonBoundaries(ErrorCell),

    #[display("No start and end times matching the lesson (at {_0}) was found.")]
    LessonTimeNotFound(CellPos),
}

pub type Result<T> = core::result::Result<T, Error>;
