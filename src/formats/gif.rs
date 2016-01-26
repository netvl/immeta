//! Metadata of GIF images.

use std::io::BufRead;
use std::borrow::Cow;
use std::str;

use byteorder::{ReadBytesExt, LittleEndian};

use types::{Result, Dimensions};
use traits::LoadableMetadata;
use utils::BufReadExt;

/// GIF file version number.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum Version {
    V87a,
    V89a
}

impl Version {
    fn from_bytes(b: &[u8]) -> Option<Version> {
        match b {
            b"87a" => Some(Version::V87a),
            b"89a" => Some(Version::V89a),
            _      => None
        }
    }
}

/// Represents various kinds of blocks which can be used in a GIF image.
#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Block {
    /// An image descriptor (image contents for one frame).
    ImageDescriptor(ImageDescriptor),
    /// Graphics control metadata block (e.g. frame delay or transparency).
    GraphicControlExtension(GraphicControlExtension),
    /// Plain text block (textual data that can be displayed as an image).
    PlainTextExtension(PlainTextExtension),
    /// Application information block (contains information about application which created the
    /// image).
    ApplicationExtension(ApplicationExtension),
    /// Comment block (contains commentary data which is not displayed in the image).
    CommentExtension(CommentExtension)
}

fn skip_blocks<R: ?Sized + BufRead, F>(r: &mut R, on_eof: F) -> Result<()>
    where F: Fn() -> Cow<'static, str>
{
    loop {
        let n = try_if_eof!(r.read_u8(), on_eof()) as u64;
        if n == 0 { return Ok(()); }
        if try!(r.skip_exact(n)) != n {
            return Err(unexpected_eof!(on_eof()));
        }
    }
}

/// Contains information about a color table (global or local).
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ColorTable {
    /// Color table size, between 2 and 256.
    pub size: u16,
    /// Whether the color table is sorted. Quoting from GIF spec:
    ///
    /// > If the flag is set, the [..] Color Table is sorted, in order of
    /// decreasing importance. Typically, the order would be decreasing frequency, with most 
    /// frequent color first. This assists a decoder, with fewer available colors, in choosing 
    /// the best subset of colors; the decoder may use an initial segment of the 
    /// table to render the graphic.
    pub sorted: bool,
}

/// Contains metadata about an image block, i.e. a single frame of a GIF image.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ImageDescriptor {
    /// Offset of the image data from the left boundary of the logical screen.
    pub left: u16,
    /// Offset of the image data from the top boundary of the logical screen.
    pub top: u16,
    /// Width of the image data.
    pub width: u16,
    /// Height of the image data.
    pub height: u16,

    /// Information about local color table, if it is present.
    pub local_color_table: Option<ColorTable>,

    /// Whether the image is interlaced.
    pub interlace: bool
}

impl ImageDescriptor {
    fn load<R: ?Sized + BufRead>(index: usize, r: &mut R) -> Result<ImageDescriptor> {
        let left = try_if_eof!(
            r.read_u16::<LittleEndian>(), 
            "when reading left offset of image block {}", index
        );
        let top = try_if_eof!(
            r.read_u16::<LittleEndian>(),
            "when reading top offset of image block {}", index
        );
        let width = try_if_eof!(
            r.read_u16::<LittleEndian>(),
            "when reading width of image block {}", index
        );
        let height = try_if_eof!(
            r.read_u16::<LittleEndian>(),
            "when reading height of image block {}", index
        );

        let packed_flags = try_if_eof!(r.read_u8(), "when reading flags of image block {}", index);
        let local_color_table        = (0b10000000 & packed_flags) > 0;
        let interlace                = (0b01000000 & packed_flags) > 0;
        let local_color_table_sorted = (0b00100000 & packed_flags) > 0;
        let local_color_table_size_p = (0b00000111 & packed_flags) >> 0;  

        let local_color_table_size = if local_color_table {
            1u16 << (local_color_table_size_p+1)
        } else {
            0
        };

        if local_color_table {
            let skip_size = local_color_table_size as u64 * 3;
            if try!(r.skip_exact(skip_size)) != skip_size {
                return Err(unexpected_eof!("when reading color table of image block {}", index));
            }
        }

        let _ = try_if_eof!(r.read_u8(), "when reading LZW minimum code size of image block {}", index);
        try!(skip_blocks(r, || format!("when reading image data of image block {}", index).into()));

        Ok(ImageDescriptor {
            left: left,
            top: top,
            width: width,
            height: height,

            local_color_table: if local_color_table {
                Some(ColorTable {
                    size: local_color_table_size,
                    sorted: local_color_table_sorted
                })
            } else { None },

            interlace: interlace
        })
    }
}

/// Contains metadata for a graphic control extension block.
///
/// This block usually leads an image descriptor block and contains information on how this
/// image should be displayed. It is especially important for animated GIF images because
/// it contains delay and disposal method flags.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct GraphicControlExtension {
    /// Indicates how the graphic should be treated after it is displayed.
    ///
    /// See `DisposalMethod` enum documentation for more information.
    pub disposal_method: DisposalMethod,
    /// Whether or not user input is required before continuing.
    ///
    /// How this flag is treated depends on a application.
    pub user_input: bool,

    /// Specifies "transparent" color in a color table, if available.
    ///
    /// "Transparent" color makes the decoder ignore a pixel and go on to the next one.
    pub transparent_color_index: Option<u8>,

    /// Defines the delay before processing the rest of the GIF stream.
    /// 
    /// The value is specified in one hundredths of a second. Use `delay_time_ms()` method
    /// to obtain a more conventional time representation.
    ///
    /// If zero, it means that there is no delay time.
    pub delay_time: u16
}

impl GraphicControlExtension {
    /// Returns delay time in milliseconds.
    ///
    /// See `delay_time` field description.
    #[inline]
    pub fn delay_time_ms(&self) -> u32 {
        self.delay_time as u32 * 10
    }

    fn load<R: ?Sized + BufRead>(index: usize, r: &mut R) -> Result<GraphicControlExtension> {
        const NAME: &'static str = "graphics control extension block";

        let block_size = try_if_eof!(r.read_u8(), "when reading block size of {} {}", NAME, index);
        if block_size != 0x04 {
            return Err(invalid_format!("invalid block size in {} {}: {}", NAME, index, block_size));
        }

        let packed_flags = try_if_eof!(r.read_u8(), "when reading flags of {} {}", NAME, index);
        let disposal_method =   (0b00011100 & packed_flags) >> 2;
        let user_input =        (0b00000010 & packed_flags) > 0;
        let transparent_color = (0b00000001 & packed_flags) > 0;

        let delay_time = try_if_eof!(
            r.read_u16::<LittleEndian>(), 
            "when reading delay time of {} {}", NAME, index
        );

        let transparent_color_index = try_if_eof!(
            r.read_u8(),
            "when reading transparent color index of {} {}", NAME, index
        );

        try!(skip_blocks(r, || format!("when reading block terminator of {} {}", NAME, index).into()));

        Ok(GraphicControlExtension {
            disposal_method: try!(
                DisposalMethod::from_u8(disposal_method)
                    .ok_or(invalid_format!("invalid disposal method in {} {}: {}", 
                                           NAME, index, disposal_method))
            ),
            user_input: user_input,
            transparent_color_index: if transparent_color { 
                Some(transparent_color_index) 
            } else { 
                None
            },
            delay_time: delay_time
        })
    }
}

/// Describes disposal methods used for GIF image frames.
///
/// Disposal method defines how the graphic should be treated after being displayed. Descriptions
/// of enum variants come from GIF spec.
#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum DisposalMethod {
    /// The decoder is not required to take any action.
    None,
    /// The graphic is to be left in place.
    DoNotDispose,
    /// The area used by the graphic must be restored to the background color.
    RestoreToBackgroundColor,
    /// The decoder is required to restore the area overwritten by the graphic with what
    /// was there prior to rendering the graphic.
    RestoreToPrevious,
    /// Unknown disposal method.
    Unknown(u8)
}

impl DisposalMethod {
    fn from_u8(n: u8) -> Option<DisposalMethod> {
        match n {
            0 => Some(DisposalMethod::None),
            1 => Some(DisposalMethod::DoNotDispose),
            2 => Some(DisposalMethod::RestoreToBackgroundColor),
            3 => Some(DisposalMethod::RestoreToPrevious),
            n if n < 8 => Some(DisposalMethod::Unknown(n)),
            _ => None
        }
    }
}

/// Contains metadata for a plain text extension block.
///
/// Plain text blocks can be used to render texts represented as an actual textual data as
/// opposed to pre-rendered rasterized text. However, it seems that these blocks are not
/// well supported by the existing software.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PlainTextExtension {
    /// Column number, in pixels, of the left edge of the text grid, with respect to 
    /// the left edge of the logical screen.
    pub left: u16,
    /// Same as above, for the top edges.
    pub top: u16,
    /// Width of the text grid in pixels.
    pub width: u16,
    /// Height of the text grid in pixels.
    pub height: u16,

    /// Width in pixels of each cell in the text grid.
    pub cell_width: u8,
    /// Height in pixels of each cell in the text grid.
    pub cell_height: u8,

    /// Index of a foreground color in the global color table.
    pub foreground_color_index: u8,
    /// Index of a background color in the global color table.
    pub background_color_index: u8
}

impl PlainTextExtension {
    fn load<R: ?Sized + BufRead>(index: usize, r: &mut R) -> Result<PlainTextExtension> {
        const NAME: &'static str = "plain text extension block";

        let block_size = try_if_eof!(r.read_u8(), "when reading block size of {} {}", NAME, index);
        if block_size != 0x0C {
            return Err(invalid_format!("invalid block size in {} {}: {}", NAME, index, block_size));
        }

        let left = try_if_eof!(
            r.read_u16::<LittleEndian>(), 
            "when reading left offset of {} {}", NAME, index
        );
        let top = try_if_eof!(
            r.read_u16::<LittleEndian>(),
            "when reading top offset of {} {}", NAME, index
        );
        let width = try_if_eof!(
            r.read_u16::<LittleEndian>(),
            "when reading width of {} {}", NAME, index
        );
        let height = try_if_eof!(
            r.read_u16::<LittleEndian>(),
            "when reading height of {} {}", NAME, index
        );

        let cell_width = try_if_eof!(
            r.read_u8(), 
            "when reading character cell width of {} {}", NAME, index
        );
        let cell_height = try_if_eof!(
            r.read_u8(), 
            "when reading character cell height of {} {}", NAME, index
        );

        let foreground_color_index = try_if_eof!(
            r.read_u8(), 
            "when reading foreground color index of {} {}", NAME, index
        );
        let background_color_index = try_if_eof!(
            r.read_u8(), 
            "when reading background color index of {} {}", NAME, index
        );

        try!(skip_blocks(r, || format!("when reading text data of {} {}", NAME, index).into()));

        Ok(PlainTextExtension {
            left: left,
            top: top,
            width: width,
            height: height,

            cell_width: cell_width,
            cell_height: cell_height,

            foreground_color_index: foreground_color_index,
            background_color_index: background_color_index
        })
    }
}

/// Contains metadata for application extension block.
///
/// These blocks usually contain information about the application which was used to create
/// the image.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ApplicationExtension {
    /// Eight ASCII bytes of an application identifier.
    pub application_identifier: [u8; 8],
    /// Three bytes of an application authentication code.
    ///
    /// Citing the GIF spec:
    ///
    /// > Sequence of three bytes used to authenticate the Application Identifier. 
    /// An Application program may use an algorithm to compute a binary code that uniquely
    /// identifies it as the application owning the Application Extension.
    pub authentication_code: [u8; 3]
}

impl ApplicationExtension {
    /// Returns application identifier as a UTF-8 string, if possible.
    ///
    /// For correct images this method should always return `Some`.
    pub fn application_identifier_str(&self) -> Option<&str> {
        str::from_utf8(&self.application_identifier).ok()
    }

    /// Returns authentication code as a UTF-8 string, if possible.
    pub fn authentication_code_str(&self) -> Option<&str> {
        str::from_utf8(&self.authentication_code).ok()
    }

    fn load<R: ?Sized + BufRead>(index: usize, r: &mut R) -> Result<ApplicationExtension> {
        const NAME: &'static str = "application extension block";

        let block_size = try_if_eof!(r.read_u8(), "when reading block size of {} {}", NAME, index);
        if block_size != 0x0B {
            return Err(invalid_format!("invalid block size in {} {}: {}", NAME, index, block_size));
        }

        let mut application_identifier = [0u8; 8];
        try!(r.read_exact(&mut application_identifier)
             .map_err(if_eof!(std, "while reading application identifier in {} {}", NAME, index)));

        let mut authentication_code = [0u8; 3];
        try!(r.read_exact(&mut authentication_code)
             .map_err(if_eof!(std, "while reading authentication code in {} {}", NAME, index)));

        try!(skip_blocks(r, || format!("when reading application data of {} {}", NAME, index).into()));

        Ok(ApplicationExtension {
            application_identifier: application_identifier,
            authentication_code: authentication_code
        })
    }
}

/// Represents a comment extension block.
///
/// Comment block does not contain any metadata, so this struct is used for uniformity
/// as a placeholder in the enum.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct CommentExtension;

impl CommentExtension {
    fn load<R: ?Sized + BufRead>(index: usize, r: &mut R) -> Result<CommentExtension> {
        const NAME: &'static str = "comments extension block";
        try!(skip_blocks(r, || format!("when reading comment data of {} {}", NAME, index).into()));

        Ok(CommentExtension)
    }
}

/// Contains metadata about the whole GIF image.
#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Metadata {
    /// GIF format version from the file header.
    pub version: Version,

    /// Logical screen dimensions of the image.
    pub dimensions: Dimensions,

    /// Information about global color table, if it is present.
    pub global_color_table: Option<ColorTable>,

    /// Number of colors available to the original image.
    ///
    /// Quoting the GIF spec:
    ///
    /// > Number of bits per primary color available to the original image, minus 1. 
    /// This value represents the size of the entire palette from which the colors in the 
    /// graphic were selected, not the number of colors actually used in the graphic. 
    /// For example, if the value in this field is 3, then the palette of the original image 
    /// had 4 bits per primary color available to create the image. This value should be set
    /// to indicate the richness of the original palette, even if not every color from the whole
    /// palette is available on the source machine.
    ///
    /// Note that the value in this structure is the number of *colors*, not the number of *bits*.
    pub color_resolution: u16,
    /// Index of the default background color in the global color table.
    pub background_color_index: u8,
    /// A factor which defines the aspect ration of a pixel in the original image.
    ///
    /// Quoting from the GIF spec:
    ///
    /// > Factor used to compute an approximation of the aspect ratio of the pixel in the original 
    /// image. If the value of the field is not 0, this approximation of the aspect ratio is 
    /// computed based on the formula: 
    /// >
    /// >    Aspect Ratio = (Pixel Aspect Ratio + 15) / 64
    /// >
    /// > The Pixel Aspect Ratio is defined to be the quotient of the pixel's width over its
    /// height. The value range in this field allows specification of the widest pixel of 4:1 to
    /// the tallest pixel of 1:4 in increments of 1/64th.
    ///
    /// If zero, no information about pixel aspect ratio is available.
    ///
    /// See also `pixel_aspect_ratio_approx()` method.
    pub pixel_aspect_ratio: u8,

    /// Metadata for each block in the GIF image.
    pub blocks: Vec<Block>
}

impl Metadata {
    /// Computes pixel aspect ratio approximation, if it is available.
    ///
    /// See `pixel_aspect_ration` field documentation.
    #[inline]
    pub fn pixel_aspect_ratio_approx(&self) -> Option<f64> {
        if self.pixel_aspect_ratio == 0 {
            None
        } else {
            Some((self.pixel_aspect_ratio as f64 + 15.0)/64.0)
        }
    }

    /// Computes the number of frames, i.e. the number of image descriptor blocks.
    #[inline]
    pub fn frames_number(&self) -> usize {
        self.blocks.iter().filter(|b| match **b {
            Block::ImageDescriptor(_) => true,
            _ => false
        }).count()
    }

    /// Returns `true` if the image is animated, `false` otherwise.
    ///
    /// This is currently decided based on the number of frames. If there are more than one frames,
    /// then the image is considered animated.
    #[inline]
    pub fn is_animated(&self) -> bool {
        // TODO: is this right?
        self.frames_number() > 1
    }
}

impl LoadableMetadata for Metadata {
    fn load<R: ?Sized + BufRead>(r: &mut R) -> Result<Metadata> {
        let mut signature = [0u8; 6];
        try!(r.read_exact(&mut signature).map_err(if_eof!(std, "when reading GIF signature")));

        let version = try!(Version::from_bytes(&signature[3..])
            .ok_or(invalid_format!("invalid GIF version: {:?}", &signature[3..])));

        let width = try_if_eof!(r.read_u16::<LittleEndian>(), "when reading logical width");
        let height = try_if_eof!(r.read_u16::<LittleEndian>(), "when reading logical height");

        let packed_flags = try_if_eof!(r.read_u8(), "when reading global flags");

        let global_color_table =        (packed_flags & 0b10000000) > 0;
        let color_resolution =          (packed_flags & 0b01110000) >> 4;
        let global_color_table_sorted = (packed_flags & 0b00001000) > 0;
        let global_color_table_size_p = (packed_flags & 0b00000111) >> 0;

        let global_color_table_size = if global_color_table {
            1u16 << (global_color_table_size_p + 1) 
        } else {
            0
        };
        let background_color_index = try_if_eof!(r.read_u8(), "when reading background color index");
        let pixel_aspect_ratio = try_if_eof!(r.read_u8(), "when reading pixel aspect ration");

        if global_color_table {
            let skip_size = global_color_table_size as u64 * 3;
            if try!(r.skip_exact(skip_size)) != skip_size {
                return Err(unexpected_eof!("when reading global color table"));
            }
        }

        let mut blocks = Vec::new();
        let mut index = 0usize;
        loop {
            let separator = try_if_eof!(r.read_u8(), "when reading separator of block {}", index);
            let block = match separator {
                0x2c => Block::ImageDescriptor(try!(ImageDescriptor::load(index, r))),
                0x21 => {
                    let label = try_if_eof!(r.read_u8(), "when reading label of block {}", index);
                    match label {
                        0x01 => Block::PlainTextExtension(try!(PlainTextExtension::load(index, r))),
                        0xf9 => Block::GraphicControlExtension(try!(GraphicControlExtension::load(index, r))),
                        0xfe => Block::CommentExtension(try!(CommentExtension::load(index, r))),
                        0xff => Block::ApplicationExtension(try!(ApplicationExtension::load(index, r))),
                        _ => return Err(invalid_format!("unknown extension type of block {}: 0x{:X}", index, label))
                    }
                },
                0x3b => break,
                _ => return Err(invalid_format!("unknown block type of block {}: 0x{:X}", index, separator))
            };
            blocks.push(block);
            index += 1;
        }

        Ok(Metadata {
            version: version,

            dimensions: (width, height).into(),

            global_color_table: if global_color_table {
                Some(ColorTable {
                    size: global_color_table_size,
                    sorted: global_color_table_sorted
                })
            } else {
                None
            },

            color_resolution: 1u16 << (color_resolution + 1),

            background_color_index: background_color_index,
            pixel_aspect_ratio: pixel_aspect_ratio,

            blocks: blocks
        })
    }
}
