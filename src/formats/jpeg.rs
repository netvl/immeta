//! Metadata of JPEG images.

use std::io::{BufReader, Read};

use byteorder::{ReadBytesExt, BigEndian};

use types::{Result, Dimensions};
use traits::LoadableMetadata;
use utils::BufReadExt;

/// Represents metadata of a JPEG image.
///
/// Currently it is very basic and only provides access to image dimensions.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Metadata {
    /// Image size.
    pub dimensions: Dimensions,
    // TODO: something else?
}

impl LoadableMetadata for Metadata {
    fn load<R: ?Sized + Read>(r: &mut R) -> Result<Metadata> {
        let mut r = &mut BufReader::new(r);
        loop {
            if try!(r.skip_until(0xff)) == 0 {
                return Err(unexpected_eof!("when searching for a marker"));
            }
            
            let marker_type = try_if_eof!(r.read_u8(), "when reading marker type");
            if marker_type == 0 { continue; }  // skip "stuffed" byte

            let has_size = match marker_type {
                0xd0...0xd9 => false,
                _ => true
            };

            let size = if has_size {
                try_if_eof!(r.read_u16::<BigEndian>(), "when reading marker payload size") - 2
            } else { 0 };

            let dimensions = match marker_type {
                0xc0 | 0xc2 => {  // maybe others?
                    // skip one byte
                    let _ = try_if_eof!(r.read_u8(), "when skipping to dimensions data");
                    let h = try_if_eof!(r.read_u16::<BigEndian>(), "when reading height");
                    let w = try_if_eof!(r.read_u16::<BigEndian>(), "when reading width");
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
