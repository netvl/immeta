use std::io::{Read, BufRead, Seek, SeekFrom};
use std::cell::{RefCell, Cell};
use std::marker::PhantomData;

use byteorder;

use types::Result;
use utils::{ByteOrder, ByteOrderReadExt};

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
            Ok(value) => value.map(Ok),
            Err(e) => Some(Err(e)),
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
            source.read_u32(self.ifds.byte_order), "when reading TIFF IFD entry data offset"
        );

        self.current_entry += 1;

        Ok(Entry {
            ifds: self.ifds,
            tag: tag,
            entry_type: entry_type.into(),
            count: count,
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
            EntryType::Unknown(_)     => None,
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
            if let Some(entry_type_size) = T::entry_type().size() {
                if entry_type_size as u32 * self.count <= 4 {
                    Some(EntryValues::Embedded(EmbeddedValues {
                        current: 0,
                        count: self.count,
                        data: self.offset,
                        _entry_type_repr: PhantomData,
                    }))
                } else {
                    Some(EntryValues::Referenced(ReferencedValues {
                        ifds: self.ifds,
                        current: 0,
                        count: self.count,
                        next_offset: self.offset,
                        _entry_type_repr: PhantomData,
                    }))
                }
            } else {
                None
            }
        } else {
            None
        }
    }

    #[inline]
    pub fn all_values<T: EntryTypeRepr>(&self) -> Option<Result<Vec<T::Repr>>> {
        if self.entry_type == T::entry_type() {
            let mut result = Vec::new();
            match T::read_many_from(&mut *self.ifds.source.borrow_mut(),
                                    self.ifds.byte_order, self.count, &mut result)
                .map_err(if_eof!("when reading TIFF IFD entry values")) {
                Ok(_) => Some(Ok(result)),
                Err(e) => Some(Err(e))
            }

        } else {
            None
        }
    }
}

pub trait EntryTypeRepr {
    type Repr;
    fn entry_type() -> EntryType;
    fn read_from<R: Read>(source: &mut R, byte_order: ByteOrder) -> byteorder::Result<(u32, Self::Repr)>;
    fn read_many_from<R: Read>(source: &mut R, byte_order: ByteOrder, n: u32, target: &mut Vec<Self::Repr>) -> byteorder::Result<()>;
    fn read_from_u32(source: u32, n: u32, count: u32) -> Option<Self::Repr>;
}

pub mod entry_types {
    use std::io::Read;
    use std::mem;
    use std::str;

    use byteorder;
    use arrayvec::ArrayVec;

    use super::{EntryType, EntryTypeRepr};
    use utils::{ByteOrder, ByteOrderReadExt};

    macro_rules! gen_entry_types {
        (
            $(
                $tpe:ident, $repr:ty,
                |$source:pat, $byte_order:pat| $read:expr,
                |$u32_source:pat, $n:pat, $count:pat| $u32_read:expr
            );+
        ) => {
            $(
                pub enum $tpe {}

                impl EntryTypeRepr for $tpe {
                    type Repr = $repr;

                    #[inline]
                    fn entry_type() -> EntryType {
                        EntryType::$tpe
                    }

                    fn read_from<R: Read>($source: &mut R, $byte_order: ByteOrder) -> byteorder::Result<(u32, $repr)> {
                        $read
                    }

                    fn read_many_from<R: Read>(source: &mut R, byte_order: ByteOrder,
                                               n: u32, target: &mut Vec<Self::Repr>) -> byteorder::Result<()> {
                        for _ in 0..n {
                            target.push(try!(Self::read_from(source, byte_order)).1);
                        }
                        Ok(())
                    }

                    fn read_from_u32($u32_source: u32, $n: u32, $count: u32) -> Option<$repr> {
                        $u32_read
                    }
                }
            )+
        }
    }

    // s = zzzzzzzz yyyyyyyy xxxxxxxx wwwwwwww
    // n =    3         2        1        0
    #[inline]
    fn nbyte(s: u32, n: u32) -> u8 {
        assert!(n <= 3);
        ((s >> 8 * (3 - n)) & 0xFF) as u8
    }

    gen_entry_types! {
        Byte, u8,
            |source, _| byteorder::ReadBytesExt::read_u8(source).map(|v| (1, v)),
            |source, n, count| if n >= count || n >= 4 { None } else { Some(nbyte(source, n)) };
        Ascii, String,
            |source, _| {
                let mut s = String::new();
                loop {
                    let b = try!(byteorder::ReadBytesExt::read_u8(source));
                    if b == 0 { break; }
                    s.push(b as char);
                }
                Ok((s.len() as u32 + 1, s))
            },
            |source, n, count| if n >= count || n >= 4 { None } else {
                // w x y z
                // +-----0   4
                // 0 +---0   4
                // +---0 0   3, 4
                // 0 +-0 0   3, 4
                // +-0 +-0   2, 4
                // +-0 0 0   2, 3, 4
                // 0 0 +-0   1, 2, 4
                // 0 0 0 0   1, 2, 3, 4
                let bs = [nbyte(source, 0), nbyte(source, 1), nbyte(source, 2), nbyte(source, 3)];
                fn find_substrings<A: Extend<(usize, usize)>>(s: &[u8], target: &mut A) {
                    let mut p = 0;
                    let mut i = 0;
                    while i < s.len() {
                        if s[i] == 0 {
                            target.extend(Some((p, i)));  // excluding zero byte
                            p = i+1;
                        }
                        i += 1;
                    }
                }
                let mut substrings = ArrayVec::<[_; 4]>::new();
                find_substrings(&bs[..count as usize], &mut substrings);
                substrings.get(n as usize)
                    .map(|&(s, e)| unsafe { str::from_utf8_unchecked(&bs[s..e]).to_owned() })
            };
        Short, u16,
            |source, byte_order| source.read_u16(byte_order).map(|v| (2, v)),
            |source, n, count| if n >= count || n >= 2 { None } else {
                Some(
                    ((nbyte(source, 2*n + 1) as u16) << 8) |
                    (nbyte(source, 2*n) as u16)
                )
            };
        Long, u32,
            |source, byte_order| source.read_u32(byte_order).map(|v| (4, v)),
            |source, n, _| if n != 1 { None } else {
                Some(
                    ((nbyte(source, 3) as u32) << 24) |
                    ((nbyte(source, 2) as u32) << 16) |
                    ((nbyte(source, 1) as u32) << 8) |
                    (nbyte(source, 0) as u32)
                )
            };
        Rational, (u32, u32),
            |source, byte_order| source.read_u32(byte_order)
                .and_then(|n| source.read_u32(byte_order).map(|d| (n, d)))
                .map(|v| (4 * 2, v)),
            |_, _, _| None;
        SignedByte, i8,
            |source, _| byteorder::ReadBytesExt::read_i8(source).map(|v| (1, v)),
            |source, n, count| if n >= count || n >= 4 { None } else { Some(nbyte(source, n) as i8) };
        Undefined, u8,
            |source, _| byteorder::ReadBytesExt::read_u8(source).map(|v| (1, v)),
            |source, n, count| if n >= count || n >= 4 { None } else { Some(nbyte(source, n)) };
        SignedShort, i16,
            |source, byte_order| source.read_i16(byte_order).map(|v| (2, v)),
            |source, n, count| if n >= count || n >= 2 { None } else {
                Some(
                    ((nbyte(source, 2*n + 1) as i16) << 8) |
                    (nbyte(source, 2*n) as i16)
                )
            };
        SignedLong, i32,
            |source, byte_order| source.read_i32(byte_order).map(|v| (4, v)),
            |source, n, _| if n >= 1 { None } else {
                Some(
                    ((nbyte(source, 3) as i32) << 24) |
                    ((nbyte(source, 2) as i32) << 16) |
                    ((nbyte(source, 1) as i32) << 8) |
                    (nbyte(source, 0) as i32)
                )
            };
        SignedRational, (i32, i32),
            |source, byte_order| source.read_i32(byte_order)
                .and_then(|n| source.read_i32(byte_order).map(|d| (n, d)))
                .map(|v| (4 * 2, v)),
            |_, _, _| None;
        Float, f32,
            |source, byte_order| source.read_f32(byte_order).map(|v| (4, v)),
            |source, n, _| if n >= 1 { None } else { Some(unsafe { mem::transmute(source) }) };
        Double, f64,
            |source, byte_order| source.read_f64(byte_order).map(|v| (8, v)),
            |_, _, _| None
    }
}

pub enum EntryValues<'a, T: EntryTypeRepr, R: BufRead + Seek + 'a> {
    #[doc(hidden)]
    Embedded(EmbeddedValues<T>),
    #[doc(hidden)]
    Referenced(ReferencedValues<'a, T, R>),
}

impl<'a, T: EntryTypeRepr, R: BufRead + Seek + 'a> Iterator for EntryValues<'a, T, R> {
    type Item = Result<T::Repr>;

    fn next(&mut self) -> Option<Result<T::Repr>> {
        match self.read_value() {
            Ok(result) => result.map(Ok),
            Err(e) => Some(Err(e))
        }
    }
}

impl<'a, T: EntryTypeRepr, R: BufRead + Seek + 'a> EntryValues<'a, T, R> {
    fn read_value(&mut self) -> Result<Option<T::Repr>> {
        match *self {
            EntryValues::Embedded(ref mut v) => Ok(v.read_value()),
            EntryValues::Referenced(ref mut v) => v.read_value(),
        }
    }
}

pub struct EmbeddedValues<T: EntryTypeRepr> {
    current: u32,
    count: u32,
    data: u32,
    _entry_type_repr: PhantomData<T>,
}

impl<T: EntryTypeRepr> EmbeddedValues<T> {
    fn read_value(&mut self) -> Option<T::Repr> {
        if self.current >= self.count {
            None
        } else {
            let result = T::read_from_u32(self.data, self.current, self.count);
            self.current += 1;
            result
        }
    }
}

pub struct ReferencedValues<'a, T: EntryTypeRepr, R: BufRead + Seek + 'a> {
    ifds: &'a LazyIfds<R>,
    current: u32,
    count: u32,
    next_offset: u32,
    _entry_type_repr: PhantomData<T>,
}

impl<'a, T: EntryTypeRepr, R: BufRead + Seek + 'a> ReferencedValues<'a, T, R> {
    fn read_value(&mut self) -> Result<Option<T::Repr>> {
        if self.current >= self.count {
            return Ok(None);
        }

        try!(self.ifds.source.borrow_mut().seek(SeekFrom::Start(self.next_offset as u64)));

        let (bytes_read, value) = try_if_eof!(
            T::read_from(&mut *self.ifds.source.borrow_mut(), self.ifds.byte_order),
            "when reading TIFF entry value"
        );
        self.next_offset += bytes_read;
        self.current += 1;

        Ok(Some(value))
    }
}
