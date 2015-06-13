use std::io::{Read, Seek, SeekFrom};
use std::fs::File;
use std::path::Path;

use types::{Result, Error};
use traits::{Metadata, LoadableMetadata};
use jpeg::JpegMetadata;

pub enum GenericMetadata {
    Jpeg(JpegMetadata)
}

pub fn load<R: ?Sized + Read + Seek>(r: &mut R) -> Result<Box<Metadata>> {
    // try jpeg
    try!(r.seek(SeekFrom::Start(0)));
    if let Ok(md) = JpegMetadata::load(r) {
        return Ok(Box::new(md));
    }

    return Err(Error::InvalidFormat);
}

pub fn load_from_file<P: AsRef<Path>>(p: P) -> Result<Box<Metadata>> {
    let mut f = try!(File::open(p));
    load(&mut f)
}
