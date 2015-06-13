use std::io::{BufReader, Read};

use byteorder::{ReadBytesExt, BigEndian};

use types::{Result, Error, Dimensions};
use traits::{Metadata, LoadableMetadata};
use utils;

pub struct JpegMetadata {
    pub dimensions: Dimensions,
    // TODO: something else?
}

impl Metadata for JpegMetadata {
    #[inline]
    fn dimensions(&self) -> Dimensions {
        self.dimensions
    }

    #[inline]
    fn bit_depth(&self) -> Option<u32> { None }

    #[inline]
    fn mime_type(&self) -> &'static str { "image/jpeg" }
}

impl LoadableMetadata for JpegMetadata {
    fn load<R: ?Sized + Read>(r: &mut R) -> Result<JpegMetadata> {
        let mut r = &mut BufReader::new(r);
        loop {
            let bytes_read = try!(utils::drop_until(r, 0xff));
            if bytes_read == 0 {
                return Err(Error::UnexpectedEndOfFile);
            }
            
            let marker_type = try!(r.read_u8());
            if marker_type == 0 { continue; }

            let has_size = match marker_type {
                0xd0...0xd9 => false,
                _ => true
            };

            let size = if has_size {
                try!(r.read_u16::<BigEndian>()) - 2
            } else { 0 };

            let dimensions = match marker_type {
                0xc0 | 0xc2 => {  // maybe others?
                    try!(r.read_u8());  // skip one byte
                    let h = try!(r.read_u16::<BigEndian>());
                    let w = try!(r.read_u16::<BigEndian>());
                    Some((w, h))
                }
                _ => None
            };

            if let Some(dimensions) = dimensions {
                return Ok(JpegMetadata {
                    dimensions: dimensions.into()
                });
            }
            
            if try!(utils::drop_bytes(r, size as u64)) != size as u64 {
                return Err(Error::UnexpectedEndOfFile);
            }
        }
    }
}



