use std::io::{Read, Seek, SeekFrom};
use std::fs::File;
use std::path::Path;

use types::{Result, Error, Dimensions};
use traits::BaseMetadata;
use jpeg::JpegMetadata;

pub enum GenericMetadata {
    Jpeg(JpegMetadata)
}

impl BaseMetadata for GenericMetadata {
    fn dimensions(&self) -> Dimensions {
        match *self {
            GenericMetadata::Jpeg(ref md) => md.dimensions
        }
    }

    #[inline]
    fn supports_load() -> bool { false }

    fn load<R: ?Sized + Read>(_: &mut R) -> Result<Self> {
        panic!("cannot load GenericMetadata from a Read; use immeta::load() function instead")
    }
}

pub fn load<R: ?Sized + Read + Seek>(r: &mut R) -> Result<GenericMetadata> {
    // try jpeg
    try!(r.seek(SeekFrom::Start(0)));
    if let Ok(md) = JpegMetadata::load(r) {
        return Ok(GenericMetadata::Jpeg(md));
    }

    return Err(Error::InvalidFormat);
}

pub fn load_from_file<P: AsRef<Path>>(p: P) -> Result<GenericMetadata> {
    let mut f = try!(File::open(p));
    load(&mut f)
}
