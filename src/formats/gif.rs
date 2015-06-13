use std::io::{BufReader, Read, BufRead};
use std::borrow::Cow;

use byteorder::{ReadBytesExt, LittleEndian};

use types::{Result, Dimensions};
use traits::Metadata as BaseMetadata;
use traits::LoadableMetadata;
use utils::{ReadExt, BufReadExt};

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

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum Block {
    ImageDescriptor(ImageDescriptor),
    GraphicControlExtension(GraphicControlExtension),
    PlainTextExtension(PlainTextExtension),
    ApplicationExtension(ApplicationExtension),
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

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ColorTable {
    pub size: u16,
    pub sorted: bool,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ImageDescriptor {
    pub left: u16,
    pub top: u16,
    pub width: u16,
    pub height: u16,

    pub local_color_table: Option<ColorTable>,

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
            let skip_size = local_color_table_size as u64;
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

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct GraphicControlExtension {
    pub disposal_method: DisposalMethod,
    pub user_input: bool,

    pub transparent_color_index: Option<u8>,

    pub delay_time: u16  // 1/100th of second
}

impl GraphicControlExtension {
    #[inline]
    pub fn delay_ms(&self) -> u32 {
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

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum DisposalMethod {
    None,
    DoNotDispose,
    RestoreToBackgroundColor,
    RestoreToPrevious
}

impl DisposalMethod {
    fn from_u8(n: u8) -> Option<DisposalMethod> {
        match n {
            0 => Some(DisposalMethod::None),
            1 => Some(DisposalMethod::DoNotDispose),
            2 => Some(DisposalMethod::RestoreToBackgroundColor),
            3 => Some(DisposalMethod::RestoreToPrevious),
            _ => None
        }
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct PlainTextExtension {
    pub left: u16,
    pub top: u16,
    pub width: u16,
    pub height: u16,

    pub cell_width: u8,
    pub cell_height: u8,

    pub foreground_color_index: u8,
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

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct ApplicationExtension {
    pub application_identifier: [u8; 8],
    pub authentication_code: [u8; 3]
}

impl ApplicationExtension {
    fn load<R: ?Sized + BufRead>(index: usize, r: &mut R) -> Result<ApplicationExtension> {
        const NAME: &'static str = "application extension block";

        let block_size = try_if_eof!(r.read_u8(), "when reading block size of {} {}", NAME, index);
        if block_size != 0x0B {
            return Err(invalid_format!("invalid block size in {} {}: {}", NAME, index, block_size));
        }

        let mut application_identifier = [0u8; 8];
        if try!(r.read_exact(&mut application_identifier)) != application_identifier.len() {
            return Err(unexpected_eof!("while reading application identifier in {} {}", NAME, index));
        }

        let mut authentication_code = [0u8; 3];
        if try!(r.read_exact(&mut authentication_code)) != authentication_code.len() {
            return Err(unexpected_eof!("while reading authentication code in {} {}", NAME, index));
        }

        try!(skip_blocks(r, || format!("when reading application data of {} {}", NAME, index).into()));

        Ok(ApplicationExtension {
            application_identifier: application_identifier,
            authentication_code: authentication_code
        })
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct CommentExtension;

impl CommentExtension {
    fn load<R: ?Sized + BufRead>(index: usize, r: &mut R) -> Result<CommentExtension> {
        const NAME: &'static str = "comments extension block";
        try!(skip_blocks(r, || format!("when reading comment data of {} {}", NAME, index).into()));

        Ok(CommentExtension)
    }
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Metadata {
    pub version: Version,

    pub dimensions: Dimensions,

    pub global_color_table: Option<ColorTable>,

    pub color_resolution: u8,
    pub background_color_index: u8,
    pub pixel_aspect_ratio: u8,

    pub blocks: Vec<Block>
}

impl Metadata {
    #[inline]
    pub fn frames_number(&self) -> usize {
        self.blocks.iter().filter(|b| match **b {
            Block::ImageDescriptor(_) => true,
            _ => false
        }).count()
    }

    #[inline]
    pub fn is_animated(&self) -> bool {
        // TODO: is this right?
        self.frames_number() > 1
    }
}

impl BaseMetadata for Metadata {
    #[inline]
    fn mime_type(&self) -> &'static str { "image/gif" }

    #[inline]
    fn dimensions(&self) -> Dimensions {
        self.dimensions
    }

    #[inline]
    fn color_depth(&self) -> Option<u8> { None }
}

impl LoadableMetadata for Metadata {
    fn load<R: ?Sized + Read>(r: &mut R) -> Result<Metadata> {
        let mut r = BufReader::new(r);

        let mut signature = [0u8; 6];
        if try!(r.read_exact(&mut signature)) != signature.len() {
            return Err(unexpected_eof!("when reading GIF signature"));
        }

        let version = try!(Version::from_bytes(&signature[3..])
            .ok_or(invalid_format!("invalid GIF version: {:?}", &signature[3..])));

        let width = try!(r.read_u16::<LittleEndian>().map_err(if_eof!("when reading logical width")));
        let height = try!(r.read_u16::<LittleEndian>().map_err(if_eof!("when reading logical height")));

        let packed_flags = try!(r.read_u8().map_err(if_eof!("when reading global flags")));

        let global_color_table =        (packed_flags & 0b10000000) > 0;
        let color_resolution =          (packed_flags & 0b01110000) >> 4;
        let global_color_table_sorted = (packed_flags & 0b00001000) > 0;
        let global_color_table_size_p = (packed_flags & 0b00000111) >> 0;

        let global_color_table_size = if global_color_table {
            1u16 << (global_color_table_size_p + 1) 
        } else {
            0
        };
        let background_color_index = try!(r.read_u8().map_err(if_eof!("when reading background color index")));
        let pixel_aspect_ratio = try!(r.read_u8().map_err(if_eof!("when reading pixel aspect ration")));

        if global_color_table {
            let skip_size = global_color_table_size as u64 * 3;
            if try!(r.skip_exact(skip_size)) != skip_size {
                return Err(unexpected_eof!("when reading global color table"));
            }
        }

        let mut blocks = Vec::new();
        let mut index = 0usize;
        loop {
            let separator = try!(r.read_u8().map_err(if_eof!("when reading separator of block {}", index)));
            let block = match separator {
                0x2c => Block::ImageDescriptor(try!(ImageDescriptor::load(index, &mut r))),
                0x21 => {
                    let label = try!(r.read_u8().map_err(if_eof!("when reading label of block {}", index)));
                    match label {
                        0x01 => Block::PlainTextExtension(try!(PlainTextExtension::load(index, &mut r))),
                        0xf9 => Block::GraphicControlExtension(try!(GraphicControlExtension::load(index, &mut r))),
                        0xfe => Block::CommentExtension(try!(CommentExtension::load(index, &mut r))),
                        0xff => Block::ApplicationExtension(try!(ApplicationExtension::load(index, &mut r))),
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

            color_resolution: color_resolution + 1,

            background_color_index: background_color_index,
            pixel_aspect_ratio: pixel_aspect_ratio,

            blocks: blocks
        })
    }
}
