use std::io::{Read, Seek, SeekFrom};
use std::fs::File;
use std::path::Path;
use std::result;

use types::{Result, Dimensions};
use traits::LoadableMetadata;
use formats::{jpeg, png, gif};

use self::GenericMetadata::*;

pub trait FromGenericMetadata: Sized {
    fn by_value(gmd: GenericMetadata) -> result::Result<Self, GenericMetadata>;
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
    pub fn dimensions(&self) -> Dimensions {
        gen_access!(self; Png, Gif, Jpeg; dimensions)
    }

    pub fn mime_type(&self) -> &'static str {
        match *self {
            Png(_) => "image/png",
            Gif(_) => "image/gif",
            Jpeg(_) => "image/jpeg"
        }
    }

    #[inline]
    pub fn downcast<T: FromGenericMetadata>(self) -> result::Result<T, GenericMetadata> {
        FromGenericMetadata::by_value(self)
    }

    #[inline]
    pub fn downcast_ref<T: FromGenericMetadata>(&self) -> Option<&T> {
        FromGenericMetadata::by_ref(self)
    }
}

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

pub fn load_from_file<P: AsRef<Path>>(p: P) -> Result<GenericMetadata> {
    let mut f = try!(File::open(p));
    load(&mut f)
}
