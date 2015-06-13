use std::io::Read;
use std::fs::File;
use std::path::Path;

use types::Result;

/// Provides several convenience functions for loading metadata from various sources.
pub trait LoadableMetadata: Sized {
    /// Loads the implementing type from the given input stream.
    fn load<R: ?Sized + Read>(r: &mut R) -> Result<Self>;

    /// Loads the implementing type from a file specified by the given path.
    ///
    /// Delegates to `load<R: Read>(&mut R)` method.
    #[inline]
    fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut f = try!(File::open(path));
        LoadableMetadata::load(&mut f)
    }

    /// Loads the implementing type from an in-memory buffer.
    ///
    /// Delegates to `load<R: Read>(&mut R)` method.
    #[inline]
    fn load_from_buf(mut buf: &[u8]) -> Result<Self> {
        LoadableMetadata::load(&mut buf)
    }
}

