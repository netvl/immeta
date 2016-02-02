use std::io::{BufReader, BufRead, Seek, Cursor};
use std::fs::File;
use std::path::Path;

use types::Result;

/// Provides several convenience functions for loading metadata from various sources.
pub trait LoadableMetadata: Sized {
    /// Loads the implementing type from the given buffered input stream.
    fn load<R: ?Sized + BufRead>(r: &mut R) -> Result<Self>;

    /// Loads the implementing type from the given buffered and seekable input stream.
    ///
    /// Delegates to `LoadableMetadata::load()` method by default. Implementations
    /// may override this behavior if their respective image format may be parsed
    /// more efficiently with seeking.
    fn load_from_seek<R: ?Sized + BufRead + Seek>(r: &mut R) -> Result<Self> {
        LoadableMetadata::load(r)
    }

    /// Loads the implementing type from a file specified by the given path.
    ///
    /// Delegates to `LoadableMetadata::load_from_seek()` method by default.
    #[inline]
    fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut f = BufReader::new(try!(File::open(path)));
        LoadableMetadata::load_from_seek(&mut f)
    }

    /// Loads the implementing type from an in-memory buffer.
    ///
    /// Delegates to `LoadableMetadata::load_from_seek()` method by default.
    #[inline]
    fn load_from_buf(buf: &[u8]) -> Result<Self> {
        LoadableMetadata::load_from_seek(&mut Cursor::new(buf))
    }
}

