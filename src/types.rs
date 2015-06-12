use std::io;
use std::result;
use std::fmt;

use num::ToPrimitive;

use byteorder;

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Dimensions {
    pub width: u32,
    pub height: u32
}

impl<T: ToPrimitive, U: ToPrimitive> From<(T, U)> for Dimensions {
    fn from((w, h): (T, U)) -> Dimensions {
        Dimensions {
            width: w.to_u32().unwrap(),
            height: h.to_u32().unwrap()
        }
    }
}

#[derive(Debug)]
pub enum Error {
    InvalidFormat,
    UnexpectedEndOfFile,
    Io(io::Error)
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::InvalidFormat => write!(f, "invalid image format"),
            Error::UnexpectedEndOfFile => write!(f, "unexpected end of file"),
            Error::Io(ref e) => write!(f, "I/O error: {}", e)
        }
    }
}

impl From<io::Error> for Error {
    #[inline]
    fn from(e: io::Error) -> Error {
        Error::Io(e)
    }
}

impl From<byteorder::Error> for Error {
    #[inline]
    fn from(e: byteorder::Error) -> Error {
        match e {
            byteorder::Error::UnexpectedEOF => Error::UnexpectedEndOfFile,
            byteorder::Error::Io(e) => Error::Io(e)
        }
    }
}

pub type Result<T> = result::Result<T, Error>;
