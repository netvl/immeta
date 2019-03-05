//! Metadata of JPEG images.

use std::io::BufRead;
use std::fmt;

use byteorder::{ReadBytesExt, BigEndian};

use types::{Result, Dimensions};
use traits::LoadableMetadata;
use utils::BufReadExt;

/// Coding process used in an image.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum CodingProcess {
    /// Sequential DCT (discrete cosine transform).
    DctSequential,
    /// Progressive DCT.
    DctProgressive,
    /// Lossless coding.
    Lossless
}

impl fmt::Display for CodingProcess {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(match *self {
            CodingProcess::DctSequential => "Sequential DCT",
            CodingProcess::DctProgressive => "Progressive DCT",
            CodingProcess::Lossless => "Lossless",
        })
    }
}

impl CodingProcess {
    fn from_marker(marker: u8) -> Option<CodingProcess> {
        match marker {
            0xc0 | 0xc1 | 0xc5 | 0xc9 | 0xcd => Some(CodingProcess::DctSequential),
            0xc2 | 0xc6 | 0xca | 0xce => Some(CodingProcess::DctProgressive),
            0xc3 | 0xc7 | 0xcb | 0xcf => Some(CodingProcess::Lossless),
            _ => None
        }
    }
}

/// Entropy coding method used in an image.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum EntropyCoding {
    /// Huffman coding.
    Huffman,
    /// Arithmetic coding.
    Arithmetic
}

impl fmt::Display for EntropyCoding {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(match *self {
            EntropyCoding::Huffman => "Huffman",
            EntropyCoding::Arithmetic => "Arithmetic",
        })
    }
}

impl EntropyCoding {
    fn from_marker(marker: u8) -> Option<EntropyCoding> {
        match marker {
            0xc0 | 0xc1 | 0xc2 | 0xc3 | 0xc5 | 0xc6 | 0xc7 => Some(EntropyCoding::Huffman),
            0xc9 | 0xca | 0xcb | 0xcd | 0xce | 0xcf => Some(EntropyCoding::Arithmetic),
            _ => None
        }
    }
}

/// Represents metadata of a JPEG image.
///
/// It provides information contained in JPEG frame header, including image dimensions,
/// coding process type and entropy coding type.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Metadata {
    /// Image size.
    pub dimensions: Dimensions,
    /// Sample precision (in bits).
    pub sample_precision: u8,
    /// Image coding process type.
    pub coding_process: CodingProcess,
    /// Image entropy coding type.
    pub entropy_coding: EntropyCoding,
    /// Whether this image uses a baseline DCT encoding.
    pub baseline: bool,
    /// Whether this image uses a differential encoding.
    pub differential: bool,
}

fn find_marker<R: ?Sized, F>(r: &mut R, name: &str, mut matcher: F) -> Result<u8>
    where R: BufRead, F: FnMut(u8) -> bool
{
    loop {
        if try!(r.skip_until(0xff)) == 0 {
            return Err(unexpected_eof!("when searching for {} marker", name));
        }
        let marker_type = try_if_eof!(r.read_u8(), "when reading marker type");
        if marker_type == 0 { continue; }  // skip "stuffed" byte

        if matcher(marker_type) {
            return Ok(marker_type);
        }
    }
}

fn skip_extra_makers<R: ?Sized + BufRead>(r: &mut R) -> Result<u8> {
    loop {
        let marker = try!(find_marker(r, "Extra", |_| true));
        if is_sof_marker(marker) {
            // return last (SOF) marker
            return Ok(marker);
        }
        let size = try_if_eof!(r.read_u16::<BigEndian>(), "when reading some marker payload size");
        let _ = r.skip_exact(size as u64 - 2);
    }
}

impl LoadableMetadata for Metadata {
    fn load<R: ?Sized + BufRead>(r: &mut R) -> Result<Metadata> {
        // read SOI marker, it must be present in all JPEG files
        try!(find_marker(r, "SOI", |m| m == 0xd8));

        let marker = try!(skip_extra_makers(r));

        // read and check SOF marker length
        let size = try_if_eof!(r.read_u16::<BigEndian>(), "when reading SOF marker payload size");
        if size <= 8 {  // 2 bytes for the length itself, 6 bytes is the minimum header size
            return Err(invalid_format!("invalid JPEG frame header size: {}", size));
        }

        // read sample precision
        let sample_precision = try_if_eof!(r.read_u8(), "when reading sample precision of the frame");

        // read height and width
        let h = try_if_eof!(r.read_u16::<BigEndian>(), "when reading JPEG frame height");
        let w = try_if_eof!(r.read_u16::<BigEndian>(), "when reading JPEG frame width");
        // TODO: handle h == 0 (we need to read a DNL marker after the first scan)

        // there is only one baseline DCT marker, naturally
        let baseline = marker == 0xc0;

        let differential = match marker {
            0xc0 | 0xc1 | 0xc2 | 0xc3 | 0xc9 | 0xca | 0xcb => false,
            0xc5 | 0xc6 | 0xc7 | 0xcd | 0xce | 0xcf => true,
            _ => unreachable!(),  // because we are inside a valid SOF marker
        };

        // unwrap can't fail, we're inside a valid SOF marker
        let coding_process = CodingProcess::from_marker(marker).unwrap();
        let entropy_coding = EntropyCoding::from_marker(marker).unwrap();

        Ok(Metadata {
            dimensions: (w, h).into(),
            sample_precision: sample_precision,
            coding_process: coding_process,
            entropy_coding: entropy_coding,
            baseline: baseline,
            differential: differential,
        })
    }
}

fn is_sof_marker(value: u8) -> bool {
    match value {
        // no 0xC4, 0xC8 and 0xCC, they are not SOF markers
        0xc0 | 0xc1 | 0xc2 | 0xc3 | 0xc5 | 0xc6 | 0xc7 | 0xc9 |
        0xca | 0xcb | 0xcd | 0xce | 0xcf => true,
        _ => false
    }
}
