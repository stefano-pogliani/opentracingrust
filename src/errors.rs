use std::io;
use std::result;
use std::sync::mpsc;

use super::span::FinishedSpan;


/// TODO
#[derive(Debug)]
pub enum Error {
    IoError(self::io::Error),
    Msg(String),
    SendError(self::mpsc::SendError<FinishedSpan>)
}


/// TODO
pub type Result<T> = self::result::Result<T, Error>;
