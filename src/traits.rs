use std::io::Read;
use std::fs::File;
use std::path::Path;

use types::{Result, Dimensions};

pub trait BaseMetadata {
    fn dimensions(&self) -> Dimensions;

    fn supports_load() -> bool;
    fn load<R: ?Sized + Read>(r: &mut R) -> Result<Self> where Self: Sized;

    #[inline]
    fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> where Self: Sized {
        let mut f = try!(File::open(path));
        BaseMetadata::load(&mut f)
    }

    #[inline]
    fn load_from_buffer(mut buf: &[u8]) -> Result<Self> where Self: Sized {
        BaseMetadata::load(&mut buf)
    }
}

pub trait BitDepthMetadata: BaseMetadata {
    fn bit_depth(&self) -> Option<u32>;
}
