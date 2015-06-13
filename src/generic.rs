use std::io::{Read, Seek, SeekFrom};
use std::fs::File;
use std::path::Path;

use types::Result;
use traits::{Metadata, LoadableMetadata};
use formats::jpeg::JpegMetadata;
use formats::png::PngMetadata;

pub enum GenericMetadata {
    Jpeg(JpegMetadata)
}

pub fn load<R: ?Sized + Read + Seek>(r: &mut R) -> Result<Box<Metadata>> {
    // try png
    try!(r.seek(SeekFrom::Start(0)));
    if let Ok(md) = PngMetadata::load(r) {
        return Ok(Box::new(md));
    }

    // try jpeg
    // should be at the bottom because JPEG can't be determined from its header (since it has none)
    try!(r.seek(SeekFrom::Start(0)));
    if let Ok(md) = JpegMetadata::load(r) {
        return Ok(Box::new(md));
    }

    return Err(invalid_format!("unknown file type"));
}

pub fn load_from_file<P: AsRef<Path>>(p: P) -> Result<Box<Metadata>> {
    let mut f = try!(File::open(p));
    load(&mut f)
}
