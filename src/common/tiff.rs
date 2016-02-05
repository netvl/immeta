use std::io::{Read, BufRead, Seek, SeekFrom};
use std::cell::{RefCell, Cell};
use std::marker::PhantomData;

use types::Result;
use utils::{ReadExt, ByteOrder, ByteOrderReadExt};

pub struct TiffReader<R: BufRead + Seek> {
    source: R
}

impl<R: BufRead + Seek> TiffReader<R> {
    pub fn new(source: R) -> TiffReader<R> {
        TiffReader {
            source: source
        }
    }

    pub fn ifds(mut self) -> Result<LazyIfds<R>> {
        let mut bom = [0u8; 2];
        try_if_eof!(std, self.source.read_exact(&mut bom), "while reading byte order mark");

        let byte_order = match &bom {
            b"II" => ByteOrder::Little,
            b"MM" => ByteOrder::Big,
            _ => return Err(invalid_format!("invalid TIFF BOM: {:?}", bom))
        };

        let magic = try_if_eof!(
            self.source.read_u16(byte_order), 
            "when reading TIFF magic number"
        );
        if magic != 42 {
            return Err(invalid_format!("invalid TIFF magic number: {}", magic));
        }

        Ok(LazyIfds {
            source: RefCell::new(self.source), 
            byte_order: byte_order,
            next_ifd_offset: Cell::new(4),
        })
    }
}

pub struct LazyIfds<R: BufRead + Seek> {
    source: RefCell<R>,
    byte_order: ByteOrder,
    next_ifd_offset: Cell<u64>,
}

impl<'a, R: BufRead + Seek> IntoIterator for &'a LazyIfds<R> {
    type Item = Result<Ifd<'a, R>>;
    type IntoIter = Ifds<'a, R>;

    fn into_iter(self) -> Ifds<'a, R> {
        Ifds(self)
    }
}

pub struct Ifds<'a, R: BufRead + Seek + 'a>(&'a LazyIfds<R>);

impl<'a, R: BufRead + Seek + 'a> Iterator for Ifds<'a, R> {
    type Item = Result<Ifd<'a, R>>;

    fn next(&mut self) -> Option<Result<Ifd<'a, R>>> {
        match self.read_ifd() {
            Err(e) => Some(Err(e)),
            Ok(Some(value)) => Some(Ok(value)),
            Ok(None) => None,
        }
    }
}

impl<'a, R: BufRead + Seek> Ifds<'a, R> {
    fn read_ifd(&mut self) -> Result<Option<Ifd<'a, R>>> {
        let next_ifd_offset = self.0.next_ifd_offset.get();

        // next ifd offset is only zero in the last entry of a TIFF document
        if next_ifd_offset == 0 {
            return Ok(None);
        }

        // seek to the beginning of the next IFD
        try_if_eof!(std,
            self.0.source.borrow_mut().seek(SeekFrom::Start(next_ifd_offset as u64)),
            "when seeking to the beginning of the next IFD"
        );
        let current_ifd_offset = next_ifd_offset;

        // read the length of this IFD
        let next_ifd_size = try_if_eof!(
            self.0.source.borrow_mut().read_u16(self.0.byte_order), "when reading number of entries in an IFD"
        );
        // it is an error for an IFD to be empty
        if next_ifd_size == 0 {
            return Err(invalid_format!("number of entries in an IFD is zero"));
        }

        // compute the offset of the next IFD offset and seek to it
        let next_ifd_offset_offset = current_ifd_offset + 2 + next_ifd_size as u64 * 12;
        try_if_eof!(std,
            self.0.source.borrow_mut().seek(SeekFrom::Start(next_ifd_offset_offset as u64)),
            "when seeking to the next IFD offset"
        );

        // read and update the next IFD offset for further calls to `next()`
        self.0.next_ifd_offset.set(try_if_eof!(
            self.0.source.borrow_mut().read_u16(self.0.byte_order), "when reading the next IFD offset"
        ) as u64);

        Ok(Some(Ifd {
            ifds: self.0,
            ifd_offset: current_ifd_offset,
            current_entry: 0,
            total_entries: next_ifd_size,
        }))
    }
}

pub struct Ifd<'a, R: BufRead + Seek + 'a> {
    ifds: &'a LazyIfds<R>,
    ifd_offset: u64,
    current_entry: u16,
    total_entries: u16,
}

impl<'a, R: BufRead + Seek + 'a> Iterator for Ifd<'a, R> {
    type Item = Result<Entry<'a, R>>;

    fn next(&mut self) -> Option<Result<Entry<'a, R>>> {
        if self.current_entry == self.total_entries {
            None
        } else {
            Some(self.read_entry())
        }
    }
}

impl<'a, R: BufRead + Seek + 'a> Ifd<'a, R> {
    fn read_entry(&mut self) -> Result<Entry<'a, R>> {
        let mut source = self.ifds.source.borrow_mut();

        // seek to the beginning of the next entry (ifd offset + 2 + next_entry * 12)
        try!(source.seek(SeekFrom::Start(self.ifd_offset + 2 + self.current_entry as u64 * 12)));

        let tag = try_if_eof!(
            source.read_u16(self.ifds.byte_order), "when reading TIFF IFD entry tag"
        );

        let entry_type = try_if_eof!(
            source.read_u16(self.ifds.byte_order), "when reading TIFF IFD entry type"
        );

        let count = try_if_eof!(
            source.read_u32(self.ifds.byte_order), "when reading TIFF IFD entry data count"
        );

        let offset = try_if_eof!(
            source.read_u32(self.ifds.byte_order), "when reading TIFF IFD offset value"
        );

        self.current_entry += 1;
        
        Ok(Entry {
            ifds: self.ifds,
            tag: tag,
            entry_type: EntryType::Byte,
            count: 0,
            offset: offset,
        })
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum EntryType {
    Byte,
    Ascii,
    Short,
    Long,
    Rational,
    SignedByte,
    Undefined,
    SignedShort,
    SignedLong,
    SignedRational,
    Float,
    Double,
    Unknown(u16),
}

impl From<u16> for EntryType {
    fn from(n: u16) -> EntryType {
        match n {
            1  => EntryType::Byte,
            2  => EntryType::Ascii,
            3  => EntryType::Short,
            4  => EntryType::Long,
            5  => EntryType::Rational,
            6  => EntryType::SignedByte,
            7  => EntryType::Undefined,
            8  => EntryType::SignedShort,
            9  => EntryType::SignedLong,
            10 => EntryType::SignedRational,
            11 => EntryType::Float,
            12 => EntryType::Double,
            n  => EntryType::Unknown(n),
        }
    }
}

impl EntryType {
    pub fn size(self) -> Option<u8> {
        match self {
            EntryType::Byte           => Some(1),
            EntryType::Ascii          => Some(1),
            EntryType::Short          => Some(2),
            EntryType::Long           => Some(4),
            EntryType::Rational       => Some(8),
            EntryType::SignedByte     => Some(1),
            EntryType::Undefined      => Some(1),
            EntryType::SignedShort    => Some(2),
            EntryType::SignedLong     => Some(4),
            EntryType::SignedRational => Some(4),
            EntryType::Float          => Some(4),
            EntryType::Double         => Some(8),
            EntryType::Unknown(n)     => None,
        }
    }
}

pub struct Entry<'a, R: BufRead + Seek + 'a> {
    ifds: &'a LazyIfds<R>,
    tag: u16,
    entry_type: EntryType,
    count: u32,
    offset: u32,
}

impl<'a, R: BufRead + Seek + 'a> Entry<'a, R> {
    #[inline]
    pub fn tag(&self) -> u16 {
        self.tag
    }

    #[inline]
    pub fn entry_type(&self) -> EntryType {
        self.entry_type
    }

    #[inline]
    pub fn count(&self) -> u32 {
        self.count
    }

    #[inline]
    pub fn values<T: EntryTypeRepr>(&self) -> Option<EntryValues<'a, T, R>> {
        if self.entry_type == T::entry_type() {
            Some(EntryValues {
                ifds: self.ifds,
                entry_type: self.entry_type,
                count: self.count,
                offset: self.offset,
                _entry_type_repr: PhantomData,
            })
        } else {
            None
        }
    }

    #[inline]
    pub fn values_vec<T: EntryTypeRepr>(&self) -> Option<Result<Vec<T::Repr>>> {
        unimplemented!()
    }
}

pub mod entry_types {
    use std::io::Read;

    use byteorder;

    use super::{EntryType, EntryTypeRepr};
    use utils::{ByteOrder, ByteOrderReadExt};
    use types::Result;

    macro_rules! gen_entry_types {
        ($($tpe:ident, $repr:ty, |$source:pat, $byte_order:pat| $read:expr);+) => {
            $(
                pub enum $tpe {}

                impl EntryTypeRepr for $tpe {
                    type Repr = $repr;

                    #[inline]
                    fn entry_type() -> EntryType {
                        EntryType::$tpe
                    }

                    fn read_from<R: Read>($source: &mut R, $byte_order: ByteOrder) -> Result<$repr> {
                        $read.map_err(From::from)
                    }
                }
            )+
        }
    }

    gen_entry_types! {
        Byte, u8,
            |source, _| byteorder::ReadBytesExt::read_u8(source);
        Ascii, String, |source, _| {
            let mut s = String::new();
            loop {
                let mut b = try!(byteorder::ReadBytesExt::read_u8(source));
                if b == 0 { break; }
                s.push(b as char);
            }
            Ok::<String, byteorder::Error>(s)
        };
        Short, u16, |source, byte_order| source.read_u16(byte_order);
        Long, u32, |source, byte_order| source.read_u32(byte_order);
        Rational, (u32, u32), |source, byte_order|
            source.read_u32(byte_order)
                .and_then(|n| source.read_u32(byte_order).map(|d| (n, d)));
        SignedByte, i8, |source, _| byteorder::ReadBytesExt::read_i8(source);
        Undefined, u8, |source, _| byteorder::ReadBytesExt::read_u8(source);
        SignedShort, i16, |source, byte_order| source.read_i16(byte_order);
        SignedLong, i32, |source, byte_order| source.read_i32(byte_order);
        SignedRational, (i32, i32), |source, byte_order| 
            source.read_i32(byte_order)
                .and_then(|n| source.read_i32(byte_order).map(|d| (n, d)));
        Float, f32, |source, byte_order| source.read_f32(byte_order);
        Double, f64, |source, byte_order| source.read_f64(byte_order)
    }
}

pub trait EntryTypeRepr {
    type Repr;
    fn entry_type() -> EntryType;
    fn read_from<R: Read>(source: &mut R, byte_order: ByteOrder) -> Result<Self::Repr>;
    fn read_many_from<R: Read>(source: &mut R, byte_order: ByteOrder, n: u32, target: Vec<Self::Repr>) -> Result<()>;
}

pub struct EntryValues<'a, T: EntryTypeRepr, R: BufRead + Seek + 'a> {
    ifds: &'a LazyIfds<R>,
    entry_type: EntryType,
    count: u32,
    offset: u32,
    _entry_type_repr: PhantomData<T>,
}

//impl<'a, R: BufRead + Seek + 'a> Iterator for EntryValues<'a, R> {
    //type Item = 
//}

