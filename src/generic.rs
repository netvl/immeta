use std::io::{Read, Cursor, Seek, SeekFrom};
use std::fs::File;
use std::path::Path;
use std::result;

use types::{Result, Dimensions};
use traits::LoadableMetadata;
use formats::{jpeg, png, gif};

use self::GenericMetadata::*;

/// Represents something which can possibly be obtained from `GenericMetadata`.
///
/// Instances of this trait are concrete metadata types. They all are embedded
/// in `GenericMetadata` structure. This trait allows a natural 
/// `generic.downcast::<specific::Metadata>` syntax to be used.
pub trait FromGenericMetadata: Sized {
    /// Obtains a value of implementing type from the `GenericMetadata` value.
    ///
    /// Returns `Ok(value)` if a value of this type is actually contained in the 
    /// provided `GenericMetadata` instance and `Err(gmd)` if not. Either way,
    /// the data is preserved from destruction and can be queried afterwards.
    fn by_value(gmd: GenericMetadata) -> result::Result<Self, GenericMetadata>;

    /// Obtains a reference to a value of implementing type from a reference to
    /// some `GenericMetadata` value.
    ///
    /// Returns `Some(reference)` if a value of this type is actually contained
    /// in the provided `GenericMetadata` instance and `None` otherwise.
    fn by_ref(gmd: &GenericMetadata) -> Option<&Self>;
}

macro_rules! impl_from_generic_metadata {
    ($t:ty, $variant:ident) => {
        impl $crate::generic::FromGenericMetadata for $t {
            fn by_value(gmd: $crate::generic::GenericMetadata) 
                -> ::std::result::Result<$t, $crate::generic::GenericMetadata>
            {
                match gmd {
                    $crate::generic::GenericMetadata::$variant(md) => Ok(md),
                    gmd => Err(gmd)
                }
            }

            fn by_ref(gmd: &$crate::generic::GenericMetadata) -> Option<&$t> {
                match *gmd {
                    $crate::generic::GenericMetadata::$variant(ref md) => Some(md),
                    _ => None
                }
            }
        }
    }
}

/// Represents metadata loaded from a file whose format was determined automatically.
///
/// Values of this type are obtained via `immeta::load()` function and its derivatives.
pub enum GenericMetadata {
    Png(png::Metadata),
    Gif(gif::Metadata),
    Jpeg(jpeg::Metadata)
}

macro_rules! gen_access {
    ($_self:ident; $($variant:ident),+; $field:ident) => {
        match *$_self {
            $($variant(ref md) => md.$field,)+
        }
    };
    ($_self:ident; $($variant:ident),+; $field:ident; $wrap:expr; $otherwise:expr) => {
        match *$_self {
            $($variant(ref md) => $wrap(md.$field),)+
            _ => $otherwise
        }
    }
}

impl GenericMetadata {
    /// Returns image dimensions from the contained metadata.
    pub fn dimensions(&self) -> Dimensions {
        gen_access!(self; Png, Gif, Jpeg; dimensions)
    }

    /// Returns a MIME type string for the image type of the contained metadata.
    pub fn mime_type(&self) -> &'static str {
        match *self {
            Png(_) => "image/png",
            Gif(_) => "image/gif",
            Jpeg(_) => "image/jpeg"
        }
    }

    /// Attemts to convert this value to the specific metadata type by value.
    ///
    /// This method is needed only to provide a convenient syntax and it is not necessary
    /// because one may just `match` on the `GenericMetadata` value.
    #[inline]
    pub fn downcast<T: FromGenericMetadata>(self) -> result::Result<T, GenericMetadata> {
        FromGenericMetadata::by_value(self)
    }

    /// Attempts to convert this value to the sepcific metadata type by reference.
    ///
    /// This method is needed only to provide a convenient syntax and it is not necessary
    /// because one may just `match` on the `GenericMetadata` value.
    #[inline]
    pub fn downcast_ref<T: FromGenericMetadata>(&self) -> Option<&T> {
        FromGenericMetadata::by_ref(self)
    }
}

/// Attempts to load metadata for an image contained in the provided input stream.
///
/// This method automatically determines the format of the contained image. Because it may
/// need to read the stream from the beginning several times, a `Seek` bound is necessary
/// on the input stream. This may cause problems only with network streams as they are
/// naturally not seekable, so one would need to buffer the data from them first.
pub fn load<R: ?Sized + Read + Seek>(r: &mut R) -> Result<GenericMetadata> {
    // try png
    try!(r.seek(SeekFrom::Start(0)));
    if let Ok(md) = png::Metadata::load(r) {
        return Ok(Png(md));
    }

    // try gif
    try!(r.seek(SeekFrom::Start(0)));
    if let Ok(md) = gif::Metadata::load(r) {
        return Ok(Gif(md));
    }

    // try jpeg
    // should be at the bottom because JPEG can't be determined from its header (since it has none)
    try!(r.seek(SeekFrom::Start(0)));
    if let Ok(md) = jpeg::Metadata::load(r) {
        return Ok(Jpeg(md));
    }

    Err(invalid_format!("unknown or unsupported file type"))
}

/// Attempts to load metadata for an image contained in a file identified by the provided path.
/// 
/// This method delegates to `load()` method and, consequently, also determines the image format
/// automatically.
pub fn load_from_file<P: AsRef<Path>>(p: P) -> Result<GenericMetadata> {
    let mut f = try!(File::open(p));
    load(&mut f)
}

/// Attempts to load metadata for an image contained in an in-memory buffer.
///
/// This method delegates to `load()` method and, consequently, also determines the image format
/// automatically.
pub fn load_from_buf(mut b: &[u8]) -> Result<GenericMetadata> {
    load(&mut Cursor::new(b))
}
