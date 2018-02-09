use std::io;
use std::num;
use std::result;

use crossbeam_channel::SendError;

use super::span::FinishedSpan;


/// Enumeration of all errors returned by OpenTracingRust.
#[derive(Debug)]
pub enum Error {
    IoError(self::io::Error),
    Msg(String),
    ParseIntError(self::num::ParseIntError),
    SendError(self::SendError<FinishedSpan>)
}

impl From<self::io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Error::IoError(error)
    }
}

impl From<self::num::ParseIntError> for Error {
    fn from(error: self::num::ParseIntError) -> Self {
        Error::ParseIntError(error)
    }
}

impl From<self::SendError<FinishedSpan>> for Error {
    fn from(error: self::SendError<FinishedSpan>) -> Self {
        Error::SendError(error)
    }
}


/// Type alias for `Result`s that can fail with an OpenTracingRust `Error`.
pub type Result<T> = self::result::Result<T, Error>;
