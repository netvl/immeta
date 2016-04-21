//! immeta allows one to load metadata from files of various image formats.
//!
//! Some kinds of applications need to work with image metadata, e.g. resolution, color depth,
//! whether the image is animated or not, etc, but do not need to access the actual image
//! contents. This library tries to provide exactly that, unifying the interface for different
//! image types under one umbrella. Because reading image metadata is far easier that
//! decoding the pixels of the image, this library can be smaller and faster and support more
//! formats than full-fledged image libraries.
//!
//! Naturally, different image formats (JPEG, PNG, GIF, WebP, etc.) all have different
//! types of metadata available within them. In fact, the only common piece of metadata
//! between all of them is image resolution (and even that is not always easily extractable,
//! as in JPEG case).
//!
//! immeta can inspect an image file and load the metadata specific to this format. Metadata 
//! for each image format is exposed as a separate type; there is also a generic type
//! which is used for dynamic image type detection. Naturally, it is possible to go from
//! the generic type to some specific type (if it is the actual image type, of course).
//!
//! Currently immeta can parse the following image formats:
//!
//!   * JPEG
//!   * PNG 1.2
//!   * GIF (both 87a and 89a)
//!
//! Support for more types will come in future versions, as well as support for particular 
//! metadata kinds (e.g. EXIF tags in JPEG) which are not yet available.
//!
//! **Important note:** this library only allows inspecting image metadata, not the image
//! contents. That is, it does not perform decoding and does not provide access to pixels
//! which the image consists of. If you need this functionality, consider using a library
//! like [image](https://crates.io/crates/image).

extern crate byteorder;
extern crate num_traits;
extern crate arrayvec;

pub use types::*;
pub use traits::*;
pub use generic::*;

#[macro_use] mod macros;
#[macro_use] mod generic;
mod traits;
mod types;
mod utils;

pub mod common;
pub mod formats;
