//! Metadata for PNG images.

use std::io::BufRead;

use byteorder::{ReadBytesExt, BigEndian};

use types::{Result, Dimensions};
use traits::LoadableMetadata;

/// Color type used in an image.
///
/// These color types directly corresponds to those defined in PNG spec.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum ColorType {
    Grayscale,
    Rgb,
    Indexed,
    GrayscaleAlpha,
    RgbAlpha
}

const CT_GRAYSCALE: u8 = 0;
const CT_RGB: u8 = 2;
const CT_INDEXED: u8 = 3;
const CT_GRAYSCALE_ALPHA: u8 = 4;
const CT_RGB_ALPHA: u8 = 6;

impl ColorType {
    fn from_u8(n: u8) -> Option<ColorType> {
        match n {
            CT_GRAYSCALE       => Some(ColorType::Grayscale),
            CT_RGB             => Some(ColorType::Rgb),
            CT_INDEXED         => Some(ColorType::Indexed),
            CT_GRAYSCALE_ALPHA => Some(ColorType::GrayscaleAlpha),
            CT_RGB_ALPHA       => Some(ColorType::RgbAlpha),
            _                  => None
        }
    }
}

fn compute_color_depth(bit_depth: u8, color_type: u8) -> Option<u8> {
    match color_type {
        CT_INDEXED => match bit_depth {
            1 | 2 | 4 | 8 => Some(bit_depth),
            _ => None
        },
        CT_GRAYSCALE => match bit_depth {
            1 | 2 | 4 | 8 | 16 => Some(bit_depth),
            _ => None,
        },
        CT_GRAYSCALE_ALPHA => match bit_depth {
            8 | 16 => Some(bit_depth*2),
            _ => None
        },
        CT_RGB => match bit_depth {
            8 | 16 => Some(bit_depth*3),
            _ => None
        },
        CT_RGB_ALPHA => match bit_depth {
            8 | 16 => Some(bit_depth*4),
            _ => None
        },
        _ => None
    }
}

/// Compression method used in an image.
///
/// PNG spec currently defines only one compression method:
///
/// > At present, only compression method 0 (deflate/inflate compression with a sliding window of
/// at most 32768 bytes) is defined.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum CompressionMethod {
    DeflateInflate
}

impl CompressionMethod {
    fn from_u8(n: u8) -> Option<CompressionMethod> {
        match n {
            0 => Some(CompressionMethod::DeflateInflate),
            _ => None
        }
    }
}

/// Filtering method used in an image.
///
/// PNG spec currently defines only one filter method:
///
/// > At present, only filter method 0 (adaptive filtering with five basic filter types) is
/// defined.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum FilterMethod {
    AdaptiveFiltering
}

impl FilterMethod {
    fn from_u8(n: u8) -> Option<FilterMethod> {
        match n {
            0 => Some(FilterMethod::AdaptiveFiltering),
            _ => None
        }
    }
}

/// Interlace method used in an image.
///
/// PNG spec says that interlacing can be disabled or Adam7 interlace method can be used.
#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub enum InterlaceMethod {
    Disabled,
    Adam7
}

impl InterlaceMethod {
    fn from_u8(n: u8) -> Option<InterlaceMethod> {
        match n {
            0 => Some(InterlaceMethod::Disabled),
            1 => Some(InterlaceMethod::Adam7),
            _ => None
        }
    }
}

/// Represents metadata of a PNG image.
#[derive(Clone, Eq, PartialEq, Debug)]
pub struct Metadata {
    /// Width and height.
    pub dimensions: Dimensions,
    /// Color type used in the image.
    pub color_type: ColorType,
    /// Color depth (bits per pixel) used in the image.
    pub color_depth: u8,
    /// Compression method used in the image.
    pub compression_method: CompressionMethod,
    /// Preprocessing method used in the image.
    pub filter_method: FilterMethod,
    /// Transmission order used in the image.
    pub interlace_method: InterlaceMethod
}

impl LoadableMetadata for Metadata {
    fn load<R: ?Sized + BufRead>(r: &mut R) -> Result<Metadata> {
        let mut signature = [0u8; 8];
        try!(r.read_exact(&mut signature).map_err(if_eof!(std, "when reading PNG signature")));

        if &signature != b"\x89PNG\r\n\x1a\n" {
            return Err(invalid_format!("invalid PNG header: {:?}", signature));
        }

        // chunk length
        let _ = try!(r.read_u32::<BigEndian>().map_err(if_eof!("when reading chunk length")));
        
        let mut chunk_type = [0u8; 4];
        try!(r.read_exact(&mut chunk_type).map_err(if_eof!(std, "when reading chunk type")));

        if &chunk_type != b"IHDR" {
            return Err(invalid_format!("invalid PNG chunk: {:?}", chunk_type));
        }

        let width = try!(r.read_u32::<BigEndian>().map_err(if_eof!("when reading width")));
        let height = try!(r.read_u32::<BigEndian>().map_err(if_eof!("when reading height")));
        let bit_depth = try!(r.read_u8().map_err(if_eof!("when reading bit depth")));
        let color_type = try!(r.read_u8().map_err(if_eof!("when reading color type")));
        let compression_method = try!(r.read_u8().map_err(if_eof!("when reading compression method")));
        let filter_method = try!(r.read_u8().map_err(if_eof!("when reading filter method")));
        let interlace_method = try!(r.read_u8().map_err(if_eof!("when reading interlace method")));

        Ok(Metadata {
            dimensions: (width, height).into(),
            color_type: try!(
                ColorType::from_u8(color_type)
                    .ok_or(invalid_format!("invalid color type: {}", color_type))
            ),
            color_depth: try!(
                compute_color_depth(bit_depth, color_type)
                    .ok_or(invalid_format!("invalid bit depth: {}", bit_depth))
            ),
            compression_method: try!(
                CompressionMethod::from_u8(compression_method)
                    .ok_or(invalid_format!("invalid compression method: {}", compression_method))
            ),
            filter_method: try!(
                FilterMethod::from_u8(filter_method)
                    .ok_or(invalid_format!("invalid filter method: {}", filter_method))
            ),
            interlace_method: try!(
                InterlaceMethod::from_u8(interlace_method)
                    .ok_or(invalid_format!("invalid interlace method: {}", interlace_method))
            )
        })
    }
}
