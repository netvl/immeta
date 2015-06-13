macro_rules! invalid_format {
    () => {
        $crate::types::Error::InvalidFormat(None)
    };
    ($s:expr) => { 
        $crate::types::Error::InvalidFormat(Some($s.into()))
    };
    ($fmt:expr, $($args:tt)*) => { 
        $crate::types::Error::InvalidFormat(Some(format!($fmt, $($args)*).into()))
    }
}

macro_rules! unexpected_eof {
    () => {
        $crate::types::Error::UnexpectedEndOfFile(None)
    };
    ($s:expr) => { 
        $crate::types::Error::UnexpectedEndOfFile(Some($s.into()))
    };
    ($fmt:expr, $($args:tt)*) => { 
        $crate::types::Error::UnexpectedEndOfFile(Some(format!($fmt, $($args)*).into()))
    }
}

macro_rules! if_eof {
    ($s:expr) => {
        |e| match e {
            ::byteorder::Error::UnexpectedEOF => unexpected_eof!($s),
            e => e.into()
        }
    };
    ($fmt:expr, $($args:tt)*) => {
        |e| match e {
            ::byteorder::Error::UnexpectedEOF => unexpected_eof!($fmt, $($args)*),
            e => e.into()
        }
    }
}

macro_rules! try_if_eof {
    ($e:expr, $s:expr) => {
        try!($e.map_err(if_eof!($s)))
    };
    ($e:expr, $fmt:expr, $($args:tt)*) => {
        try!($e.map_err(if_eof!($fmt, $($args)*)))
    }
}
