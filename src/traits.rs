use std::io::Read;
use std::fs::File;
use std::path::Path;

use types::Result;

pub trait LoadableMetadata: Sized {
    fn load<R: ?Sized + Read>(r: &mut R) -> Result<Self>;

    #[inline]
    fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut f = try!(File::open(path));
        LoadableMetadata::load(&mut f)
    }

    #[inline]
    fn load_from_buffer(mut buf: &[u8]) -> Result<Self> {
        LoadableMetadata::load(&mut buf)
    }
}

