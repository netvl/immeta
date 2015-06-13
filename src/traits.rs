use std::io::Read;
use std::fs::File;
use std::path::Path;
use std::any::{Any, TypeId};
use std::mem;
use std::raw;
use std::result;

use types::{Result, Dimensions};

pub trait Metadata: Any + Send {
    fn mime_type(&self) -> &'static str;
    fn dimensions(&self) -> Dimensions;
}

impl Metadata {
    pub fn is<T: Any>(&self) -> bool {
        let t = TypeId::of::<T>();
        let s = self.get_type_id();
        t == s
    }
    
    pub fn downcast_ref<T: Any>(&self) -> Option<&T> {
        if self.is::<T>() {
            unsafe {
                let to: raw::TraitObject = mem::transmute(self);
                let res: &T = mem::transmute(to.data);
                Some(res)
            }
        } else {
            None
        }
    }
}

pub trait MetadataBox {
    fn downcast<T: Any>(self) -> result::Result<Box<T>, Self>;
}

impl MetadataBox for Box<Metadata> {
    fn downcast<T: Any>(self) -> result::Result<Box<T>, Box<Metadata>> {
        if self.is::<T>() {
            unsafe {
                let raw = mem::transmute::<Box<Metadata>, *mut Metadata>(self);
                let to = mem::transmute::<*mut Metadata, raw::TraitObject>(raw);
                Ok(Box::from_raw(to.data as *mut T))
            }
        } else {
            Err(self)
        }
    }
}

pub trait LoadableMetadata: Metadata {
    fn load<R: ?Sized + Read>(r: &mut R) -> Result<Self> where Self: Sized;

    #[inline]
    fn load_from_file<P: AsRef<Path>>(path: P) -> Result<Self> where Self: Sized {
        let mut f = try!(File::open(path));
        LoadableMetadata::load(&mut f)
    }

    #[inline]
    fn load_from_buffer(mut buf: &[u8]) -> Result<Self> where Self: Sized {
        LoadableMetadata::load(&mut buf)
    }
}
