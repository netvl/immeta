use std::io::{BufReader, Read};

use byteorder::{ReadBytesExt, BigEndian};

use types::{Result, Dimensions};
use traits::Metadata as BaseMetadata;
use traits::LoadableMetadata;
use utils::BufReadExt;

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Metadata {
    pub dimensions: Dimensions,
    // TODO: something else?
}

impl BaseMetadata for Metadata {
    #[inline]
    fn mime_type(&self) -> &'static str { "image/jpeg" }

    #[inline]
    fn dimensions(&self) -> Dimensions {
        self.dimensions
    }

    #[inline]
    fn color_depth(&self) -> Option<u8> { None }
}

impl LoadableMetadata for Metadata {
    fn load<R: ?Sized + Read>(r: &mut R) -> Result<Metadata> {
        let mut r = &mut BufReader::new(r);
        loop {
            let bytes_read = try!(r.skip_until(0xff));
            if bytes_read == 0 {
                return Err(unexpected_eof!("when searching for a marker"));
            }
            
            let marker_type = try!(r.read_u8().map_err(if_eof!("when reading marker type")));
            if marker_type == 0 { continue; }  // skip "stuffed" byte

            let has_size = match marker_type {
                0xd0...0xd9 => false,
                _ => true
            };

            let size = if has_size {
                try!(r.read_u16::<BigEndian>()
                    .map_err(if_eof!("when reading marker payload size"))) - 2
            } else { 0 };

            let dimensions = match marker_type {
                0xc0 | 0xc2 => {  // maybe others?
                    // skip one byte
                    try!(r.read_u8().map_err(if_eof!("when skipping to dimensions data")));
                    let h = try!(r.read_u16::<BigEndian>().map_err(if_eof!("when reading height")));
                    let w = try!(r.read_u16::<BigEndian>().map_err(if_eof!("when reading width")));
                    Some((w, h))
                }
                _ => None
            };

            if let Some(dimensions) = dimensions {
                return Ok(Metadata {
                    dimensions: dimensions.into()
                });
            }
            
            let size = size as u64;
            if try!(r.skip_exact(size)) != size {
                return Err(unexpected_eof!("when skipping marker payload"));
            }
        }
    }
}



