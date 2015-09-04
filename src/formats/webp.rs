use std::io::Read;

use types::{Result, Dimensions};
use common::riff::{RiffReader, RiffChunk, ChunkId};
use traits::LoadableMetadata;
use utils::ReadExt;

#[derive(Clone, Eq, PartialEq, Debug)]
pub enum Metadata {
    VP8(VP8Metadata),
    VP8L(VP8LMetadata),
    VP8X(VP8XMetadata)
}

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct VP8Metadata {
    pub version_number: u8,
    pub show_frame: bool,
    pub first_partition_len: u32,
    pub frame: VP8Frame
}

#[derive(Clone, Eq, PartialEq, Debug)]
pub enum VP8Frame {
    Key { dimensions: Dimensions, x_scale: u8, y_scale: u8 },
    Inter
}

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct VP8LMetadata;

#[derive(Clone, Eq, PartialEq, Debug)]
pub struct VP8XMetadata;

const WEBP_CHUNK_TYPE: ChunkId = ChunkId([b'W', b'E', b'B', b'P']);
const ALPH_CHUNK_ID: ChunkId   = ChunkId([b'A', b'L', b'P', b'H']);
const VP8_CHUNK_ID: ChunkId    = ChunkId([b'V', b'P', b'8', b' ']);
const VP8L_CHUNK_ID: ChunkId   = ChunkId([b'V', b'P', b'8', b'L']);
const VP8X_CHUNK_ID: ChunkId   = ChunkId([b'V', b'P', b'8', b'X']);

impl Metadata {
    pub fn dimensions(&self) -> Dimensions {
        match *self {
            Metadata::VP8(VP8Metadata { frame: VP8Frame::Key { dimensions, .. }, .. }) => dimensions,
            _ => unimplemented!()
        }
    }
}

impl LoadableMetadata for Metadata {
    fn load<R: ?Sized + Read>(r: &mut R) -> Result<Metadata> {
        let mut rr = RiffReader::new(r);

        let mut root = try!(rr.root());
        if root.chunk_type() != WEBP_CHUNK_TYPE {
            return Err(invalid_format!("invalid WEBP signature"));
        }

        loop {
            let mut chunk = match root.next() {
                Some(c) => try!(c),
                None => return Err(unexpected_eof!("when reading first WEBP chunk"))
            };

            match chunk.chunk_id() {
                VP8_CHUNK_ID => return read_vp8_chunk(&mut chunk).map(Metadata::VP8),
                VP8L_CHUNK_ID => unimplemented!(),
                VP8X_CHUNK_ID => unimplemented!(),
                ALPH_CHUNK_ID => unimplemented!(),
                cid => return Err(invalid_format!("invalid WEBP chunk id: {}", cid))
            }
        }
    }
}

fn read_vp8_chunk(chunk: &mut RiffChunk) -> Result<VP8Metadata> {
    let r = chunk.contents();

    let mut hdr = [0u8; 3];
    if try!(r.read_exact_0(&mut hdr)) != 3 {
        return Err(unexpected_eof!("when reading VP8 frame header"));
    }

    let mut result = VP8Metadata {
        version_number: 0,
        show_frame: false,
        first_partition_len: 0,
        frame: VP8Frame::Inter
    };

    // bits of first three bytes:
    //    xxxsvvvf xxxxxxxx xxxxxxxx
    // where
    //    f  --  frame type, 0 is key frame, 1 is interframe
    //    v  --  version number
    //    s  --  show frame flag, 1 is display, 0 is don't display
    //    x  --  size of first data partition in bytes

    let key_frame = hdr[0] & 1 == 0;
    result.version_number = (hdr[0] >> 1) & 7;
    result.show_frame = (hdr[0] >> 4) & 1 == 1;
    result.first_partition_len = ((hdr[0] >> 5) as u32) | 
                                 ((hdr[1] as u32) << 3) | 
                                 ((hdr[2] as u32) << 11);

    if key_frame {
        let mut hdr = [0u8; 7];
        if try!(r.read_exact_0(&mut hdr)) != 7 {
            return Err(unexpected_eof!("when reading VP8 key frame header"));
        }

        // check magic value
        if &hdr[..3] != &[0x9d, 0x01, 0x2a] {
            return Err(invalid_format!("VP8 key frame magic code is invalid: {:?}", &hdr[..3]));
        }

        // bits of next four bytes:
        //    wwwwwwww xxwwwwww hhhhhhhh yyhhhhhh
        // where
        //    x  --  horizontal scale
        //    w  --  width
        //    y  --  vertical scale
        //    h  --  height

        let width  = ((hdr[4] & 0x3f) as u32) << 8 | hdr[3] as u32;
        let height = ((hdr[6] & 0x3f) as u32) << 8 | hdr[5] as u32;
        let x_scale = hdr[4] >> 6;
        let y_scale = hdr[6] >> 6;

        result.frame = VP8Frame::Key {
            dimensions: (width, height).into(),
            x_scale: x_scale,
            y_scale: y_scale
        };
    }

    Ok(result)
}

