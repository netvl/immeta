use std::io::Read;
use std::fs::File;
use std::path::Path;
use std::any::Any;

use types::{Result, Dimensions};

pub trait Metadata: Any {
    fn dimensions(&self) -> Dimensions;
    fn bit_depth(&self) -> Option<u32>;
    fn mime_type(&self) -> &'static str;
}

pub trait LoadableMetadata: Metadata {
    fn load<R: ?Sized + Read>(r: &mut R) -> Result<Self> where Self: Sized;

    #[inline]
    fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> where Self: Sized {
        let mut f = try!(File::open(path));
        LoadableMetadata::load(&mut f)
    }

    #[inline]
    fn load_from_buffer(mut buf: &[u8]) -> Result<Self> where Self: Sized {
        LoadableMetadata::load(&mut buf)
    }
}
