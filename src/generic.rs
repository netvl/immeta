use std::io::{Read, Cursor, Seek, SeekFrom};
use std::fs::File;
use std::path::Path;
use std::result;

use types::{Result, Dimensions};
use traits::LoadableMetadata;
use formats::{jpeg, png, gif};
use generic::markers::MetadataMarker;

/// Contains metadata marker types.
///
/// Metadata markers is a convenient way to access metadata loading functions for particular
/// image types. They are also integrated with `GenericMetadata`, providing a convenient
/// syntax to downcast a `GenericMetadata` value to a specific metadata type.
///
/// Metadata marker types can be used directly, for example:
/// ```ignore
/// use immeta::markers::Jpeg;
///
/// let metadata = Jpeg::load_from_file("kitty.jpg").unwrap();
/// ```
///
/// They can also be used together with `GenericMetadata`:
/// ```ignore
/// use immeta::markers::Jpeg;
///
/// let gmd = immeta::load_from_file("kitty.jpg").unwrap();
/// let jpeg_metadata: Jpeg::Metadata = gmd.into::<Jpeg>().unwrap();
/// ```
///
/// Alternatively, you can use `as_ref()`:
/// ```ignore
/// let jpeg_metadata: &Jpeg::Metadata = gmd.as_ref::<Jpeg>().unwrap();
/// ```
///
/// `MetadataMarker::Metadata` associated type always points to concrete metadata type
/// from one of `immeta::formats` submodule.
pub mod markers {
    use std::io::Read;
    use std::path::Path;
    use std::result;

    use generic::GenericMetadata;
    use types::Result;
    use formats::{jpeg, png, gif};

    /// A marker trait for specific metadata type.
    pub trait MetadataMarker {
        type Metadata;

        /// Tries to convert the given `GenericMetadata` instance into a concrete metadata type.
        ///
        /// If the generic value really contains the associated metadata type, then `Ok` variant
        /// is returned; otherwise `Err` variant containing the original value is returned.
        ///
        /// # Examples
        ///
        /// ```no_run
        /// use immeta::markers::Jpeg;
        /// use immeta::formats::jpeg;
        /// use immeta::GenericMetadata;
        ///
        /// let generic = immeta::load_from_file("kitty.jpg").unwrap();
        /// let concrete: Result<jpeg::Metadata, GenericMetadata> = generic.into::<Jpeg>();
        /// assert!(concrete.is_ok());
        /// ```
        /// 
        /// ```no_run
        /// use immeta::markers::Jpeg;
        /// use immeta::formats::jpeg;
        /// use immeta::GenericMetadata;
        ///
        /// let generic = immeta::load_from_file("kitty.png").unwrap();
        /// let concrete: Result<jpeg::Metadata, GenericMetadata> = generic.into::<Jpeg>();
        /// assert!(concrete.is_err());
        /// ```
        fn from_generic(gmd: GenericMetadata) -> result::Result<Self::Metadata, GenericMetadata>;

        /// Tries to extract a reference to a concrete metadata type from the given
        /// `GenericMetadata` reference.
        ///
        /// Behaves similarly to `from_generic()`, except using references instead of immediate
        /// values.
        ///
        /// # Examples
        ///
        /// ```no_run
        /// use immeta::markers::Jpeg;
        /// use immeta::formats::jpeg;
        ///
        /// let generic = immeta::load_from_file("kitty.jpg").unwrap();
        /// let concrete: Option<&jpeg::Metadata> = generic.as_ref::<Jpeg>();
        /// assert!(concrete.is_some());
        /// ```
        ///
        /// ```no_run
        /// use immeta::markers::Jpeg;
        /// use immeta::formats::jpeg;
        ///
        /// let generic = immeta::load_from_file("kitty.png").unwrap();
        /// let concrete: Option<&jpeg::Metadata> = generic.as_ref::<Jpeg>();
        /// assert!(concrete.is_none());
        /// ```
        fn from_generic_ref(gmd: &GenericMetadata) -> Option<&Self::Metadata>;

        /// Attempts to load metadata for an image of a concrete type from the provided reader.
        ///
        /// Invokes `LoadableMetadata::load()` for the associated metadata type. Use this
        /// method instead of calling `load()` on the metadata type directly.
        ///
        /// # Examples
        ///
        /// ```no_run
        /// use std::io;
        /// use immeta::markers::{MetadataMarker, Jpeg};
        ///
        /// let data = io::stdin();
        /// let metadata = Jpeg::load(&mut data.lock());
        /// ```
        fn load<R: ?Sized + Read>(r: &mut R) -> Result<Self::Metadata>;

        /// Attempts to load metadata for an image of a concrete type from a file identified
        /// by the provided path.
        ///
        /// Invokes `LoadableMetadata::load_from_file()` for the associated metadata type. Use this
        /// method instead of calling `load_from_file()` on the metadata type directly.
        ///
        /// # Examples
        ///
        /// ```no_run
        /// use immeta::markers::{MetadataMarker, Jpeg};
        ///
        /// let metadata = Jpeg::load_from_file("kitty.jpg");
        /// ```
        fn load_from_file<P: AsRef<Path>>(p: P) -> Result<Self::Metadata>;

        /// Attempts to load metadata for an image of a concrete type from the provided byte
        /// buffer.
        ///
        /// Invokes `LoadableMetadata::load_from_buf()` for the associated metadata type. Use this
        /// method instead of calling `load_from_buf()` on the metadata type directly.
        ///
        /// # Examples
        /// 
        /// ```no_run
        /// use immeta::markers::{MetadataMarker, Jpeg};
        ///
        /// let buf: &[u8] = &[1, 2, 3, 4];   // pretend that this is an actual image
        /// let metadata = Jpeg::load_from_buf(buf);
        /// ```
        fn load_from_buf(b: &[u8]) -> Result<Self::Metadata>;
    }

    macro_rules! impl_metadata_marker {
        ($name:ident, $gvar:ident, $mtpe:ty) => {
            pub enum $name {}

            impl MetadataMarker for $name {
                type Metadata = $mtpe;
        
                #[inline]
                fn from_generic(gmd: GenericMetadata) -> result::Result<$mtpe, GenericMetadata> {
                    match gmd {
                        $crate::generic::GenericMetadata::$gvar(md) => Ok(md),
                        gmd => Err(gmd)
                    }
                }

                #[inline]
                fn from_generic_ref(gmd: &GenericMetadata) -> Option<&$mtpe> {
                    match *gmd {
                        $crate::generic::GenericMetadata::$gvar(ref md) => Some(md),
                        _ => None
                    }
                }

                #[inline]
                fn load<R: ?Sized + Read>(r: &mut R) -> Result<$mtpe> {
                    $crate::traits::LoadableMetadata::load(r)
                }

                #[inline]
                fn load_from_file<P: AsRef<Path>>(p: P) -> Result<$mtpe> {
                    $crate::traits::LoadableMetadata::load_from_file(p)
                }

                #[inline]
                fn load_from_buf(b: &[u8]) -> Result<$mtpe> {
                    $crate::traits::LoadableMetadata::load_from_buf(b)
                }
            }
        }
    }

    impl_metadata_marker! { Jpeg, Jpeg, jpeg::Metadata }
    impl_metadata_marker! { Png, Png, png::Metadata }
    impl_metadata_marker! { Gif, Gif, gif::Metadata }
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
            $($crate::generic::GenericMetadata::$variant(ref md) => md.$field,)+
        }
    };
    ($_self:ident; $($variant:ident),+; $field:ident; $wrap:expr; $otherwise:expr) => {
        match *$_self {
            $($crate::generic::GenericMetadata::$variant(ref md) => $wrap(md.$field),)+
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
            GenericMetadata::Png(_) => "image/png",
            GenericMetadata::Gif(_) => "image/gif",
            GenericMetadata::Jpeg(_) => "image/jpeg"
        }
    }

    /// Attemts to convert this value to the specific metadata type by value.
    ///
    /// This method is needed only to provide a convenient syntax and it is not necessary
    /// because one may just `match` on the `GenericMetadata` value.
    #[inline]
    pub fn into<T: MetadataMarker>(self) -> result::Result<T::Metadata, GenericMetadata> {
        <T as MetadataMarker>::from_generic(self)
    }

    /// Attempts to convert this value to the sepcific metadata type by reference.
    ///
    /// This method is needed only to provide a convenient syntax and it is not necessary
    /// because one may just `match` on the `GenericMetadata` value.
    #[inline]
    pub fn as_ref<T: MetadataMarker>(&self) -> Option<&T::Metadata> {
        <T as MetadataMarker>::from_generic_ref(self)
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
        return Ok(GenericMetadata::Png(md));
    }

    // try gif
    try!(r.seek(SeekFrom::Start(0)));
    if let Ok(md) = gif::Metadata::load(r) {
        return Ok(GenericMetadata::Gif(md));
    }

    // try jpeg
    // should be the last because JPEG can't be determined from its header (since it has none)
    try!(r.seek(SeekFrom::Start(0)));
    if let Ok(md) = jpeg::Metadata::load(r) {
        return Ok(GenericMetadata::Jpeg(md));
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
pub fn load_from_buf(b: &[u8]) -> Result<GenericMetadata> {
    load(&mut Cursor::new(b))
}
