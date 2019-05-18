use std::error::Error as StdError;
use std::fmt;
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

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Error::IoError(ref io) => fmt::Display::fmt(io, f),
            Error::Msg(ref msg) => fmt::Display::fmt(msg, f),
            Error::ParseIntError(ref parse) => fmt::Display::fmt(parse, f),
            Error::SendError(ref send) => fmt::Display::fmt(send, f),
        }
    }
}

impl StdError for Error {}

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
