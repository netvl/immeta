use std::io::{Read, Seek, SeekFrom};
use std::fs::File;
use std::path::Path;

use types::Result;
use traits::{Metadata, LoadableMetadata};
use formats::{jpeg, png, gif};

pub fn load<R: ?Sized + Read + Seek>(r: &mut R) -> Result<Box<Metadata>> {
    // try png
    try!(r.seek(SeekFrom::Start(0)));
    if let Ok(md) = png::Metadata::load(r) {
        return Ok(Box::new(md));
    }

    // try gif
    try!(r.seek(SeekFrom::Start(0)));
    if let Ok(md) = gif::Metadata::load(r) {
        return Ok(Box::new(md));
    }

    // try jpeg
    // should be at the bottom because JPEG can't be determined from its header (since it has none)
    try!(r.seek(SeekFrom::Start(0)));
    if let Ok(md) = jpeg::Metadata::load(r) {
        return Ok(Box::new(md));
    }

    return Err(invalid_format!("unknown or unsupported file type"));
}

pub fn load_from_file<P: AsRef<Path>>(p: P) -> Result<Box<Metadata>> {
    let mut f = try!(File::open(p));
    load(&mut f)
}
