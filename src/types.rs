use std::io;
use std::result;
use std::fmt;
use std::borrow::Cow;
use std::error;

use num::ToPrimitive;

/// Library-specific error type which is returned by metadata loading operations.
#[derive(Debug)]
pub enum Error {
    /// Returned when metadata can't be recovered because image format is invalid.
    ///
    /// This error can be caused by broken file or when trying to load an image with
    /// an incorrect metadata decoder, e.g. trying to load PNG metadata from JPEG.
    InvalidFormat(Cow<'static, str>),

    /// Returned when metadata can't be recovered because of the sudden end of the image file.
    ///
    /// Usually this error is caused by broken files, but it may also be cause by applying
    /// loose formats (like JPEG) to a different image type.
    UnexpectedEndOfFile(Option<Cow<'static, str>>),

    /// Returned when an I/O error occurs when reading an input stream.
    Io(io::Error)
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::InvalidFormat(ref s) => write!(f, "invalid image format: {}", s),
            Error::UnexpectedEndOfFile(None) => write!(f, "unexpected end of file"),
            Error::UnexpectedEndOfFile(Some(ref s)) => write!(f, "unexpected end of file: {}", s),
            Error::Io(ref e) => write!(f, "I/O error: {}", e)
        }
    }
}

impl error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::InvalidFormat(_) => "invalid image format",
            Error::UnexpectedEndOfFile(_) => "unexpected end of file",
            Error::Io(_) => "i/o error"
        }
    }

    fn cause(&self) -> Option<&error::Error> {
        match *self {
            Error::Io(ref e) => Some(e),
            _ => None
        }
    }
}

impl From<io::Error> for Error {
    #[inline]
    fn from(e: io::Error) -> Error {
        Error::Io(e)
    }
}

impl From<::byteorder::Error> for Error {
    #[inline]
    fn from(e: ::byteorder::Error) -> Error {
        match e {
            ::byteorder::Error::UnexpectedEOF => Error::UnexpectedEndOfFile(None),
            ::byteorder::Error::Io(e) => Error::Io(e)
        }
    }
}

/// Library-specific result type.
pub type Result<T> = result::Result<T, Error>;

/// Represents image dimensions in pixels.
///
/// As it turns out, this is essentially the only common piece of information across
/// various image formats.
///
/// It is possible to convert pairs of type `(T1, T2)`, where `T1` and `T2` are primitive
/// number types, to this type, however, this is mostly needed for internal usage.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub struct Dimensions {
    /// Image width in pixels.
    pub width: u32,
    /// Image height in pixels.
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
