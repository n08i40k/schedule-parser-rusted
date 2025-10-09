use crate::parser::worksheet::CellPos;
use derive_more::{Display, Error, From};

#[derive(Debug, Display, Error, From)]
pub enum Error {
    #[from]
    BadXls(calamine::XlsError),

    #[display("No work sheets found.")]
    NoWorkSheets,

    #[display("There is no data on work sheet boundaries.")]
    UnknownWorkSheetRange,

    #[display("Failed to read lesson start and end of lesson at {_0}.")]
    NoLessonBoundaries(CellPos),

    #[display("No start and end times matching the lesson (at {_0}) was found.")]
    LessonTimeNotFound(CellPos),
}

pub type Result<T> = core::result::Result<T, Error>;
