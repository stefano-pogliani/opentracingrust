use std::io;
use std::num;
use std::result;
use std::sync::mpsc;

use super::span::FinishedSpan;


/// TODO
#[derive(Debug)]
pub enum Error {
    IoError(self::io::Error),
    Msg(String),
    ParseIntError(self::num::ParseIntError),
    SendError(self::mpsc::SendError<FinishedSpan>)
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


/// TODO
pub type Result<T> = self::result::Result<T, Error>;
