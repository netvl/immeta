use std::io::{Read, Seek, SeekFrom};
use std::cell::{RefCell, Cell};
use std::marker::PhantomData;

use byteorder;

use types::Result;
use utils::{ByteOrder, ByteOrderReadExt};

/// A TIFF document reader.
///
/// This structure wraps a `Read` and `Seek` implementation and allows one to read a TIFF
/// document from it.
pub struct TiffReader<R: Read + Seek> {
    source: R
}

impl<R: Read + Seek> TiffReader<R> {
    /// Wraps the provider `Read + Seek` implementation and returns a new TIFF reader.
    pub fn new(source: R) -> TiffReader<R> {
        TiffReader {
            source: source
        }
    }

    /// Returns an iterator over IFDs in the TIFF document.
    ///
    /// This method first checks that the underlying data stream is indeed a valid TIFF document,
    /// and only then returns the iterator.
    ///
    /// Note that the returned value does not implement `IntoIterator`, but an immutable
    /// reference to it does. Therefore, it should be used like this:
    ///
    /// ```no_run
    /// # use std::io::Cursor;
    /// # use immeta::common::tiff::TiffReader;
    /// # let r = TiffReader::new(Cursor::new(Vec::<u8>::new()));
    /// for ifd in &r.ifds().unwrap() {
    ///     // ...
    /// }
    /// ```
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

        let next_ifd_offset = try_if_eof!(
            self.source.read_u32(byte_order),
            "when reading first TIFF IFD offset"
        );

        Ok(LazyIfds {
            source: RefCell::new(self.source),
            byte_order: byte_order,
            next_ifd_offset: Cell::new(next_ifd_offset as u64),
        })
    }
}

/// An intermediate structure, a reference to which can be converted to an iterator
/// of IFDs.
pub struct LazyIfds<R: Read + Seek> {
    source: RefCell<R>,
    byte_order: ByteOrder,
    next_ifd_offset: Cell<u64>,
}

impl<'a, R: Read + Seek> IntoIterator for &'a LazyIfds<R> {
    type Item = Result<Ifd<'a, R>>;
    type IntoIter = Ifds<'a, R>;

    fn into_iter(self) -> Ifds<'a, R> {
        Ifds(self)
    }
}

/// An iterator of IFDs in a TIFF document.
pub struct Ifds<'a, R: Read + Seek + 'a>(&'a LazyIfds<R>);

impl<'a, R: Read + Seek + 'a> Iterator for Ifds<'a, R> {
    type Item = Result<Ifd<'a, R>>;

    fn next(&mut self) -> Option<Result<Ifd<'a, R>>> {
        match self.read_ifd() {
            Ok(value) => value.map(Ok),
            Err(e) => Some(Err(e)),
        }
    }
}

impl<'a, R: Read + Seek> Ifds<'a, R> {
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
        let current_ifd_size = try_if_eof!(
            self.0.source.borrow_mut().read_u16(self.0.byte_order), "when reading number of entries in an IFD"
        );
        // it is an error for an IFD to be empty
        if current_ifd_size == 0 {
            return Err(invalid_format!("number of entries in an IFD is zero"));
        }

        // compute the offset of the next IFD offset and seek to it
        let next_ifd_offset_offset = current_ifd_offset + 2 + current_ifd_size as u64 * 12;
        try_if_eof!(std,
            self.0.source.borrow_mut().seek(SeekFrom::Start(next_ifd_offset_offset as u64)),
            "when seeking to the next IFD offset"
        );

        // read and update the next IFD offset for further calls to `next()`
        self.0.next_ifd_offset.set(try_if_eof!(
            self.0.source.borrow_mut().read_u32(self.0.byte_order), "when reading the next IFD offset"
        ) as u64);

        Ok(Some(Ifd {
            ifds: self.0,
            ifd_offset: current_ifd_offset,
            current_entry: 0,
            total_entries: current_ifd_size,
        }))
    }
}

/// Represents a single IFD.
///
/// A TIFF IFD consists of entries, so this structure is an iterator yielding IFD entries.
pub struct Ifd<'a, R: Read + Seek + 'a> {
    ifds: &'a LazyIfds<R>,
    ifd_offset: u64,
    current_entry: u16,
    total_entries: u16,
}

impl<'a, R: Read + Seek + 'a> Iterator for Ifd<'a, R> {
    type Item = Result<Entry<'a, R>>;

    fn next(&mut self) -> Option<Result<Entry<'a, R>>> {
        if self.current_entry == self.total_entries {
            None
        } else {
            Some(self.read_entry())
        }
    }
}

impl<'a, R: Read + Seek + 'a> Ifd<'a, R> {
    #[inline]
    fn len(&self) -> u16 {
        self.total_entries
    }

    fn read_entry(&mut self) -> Result<Entry<'a, R>> {
        let mut source = self.ifds.source.borrow_mut();

        // seek to the beginning of the next entry (ifd offset + 2 + next_entry * 12)
        try!(source.seek(SeekFrom::Start(self.ifd_offset + 2 + self.current_entry as u64 * 12)));

        // read the tag
        let tag = try_if_eof!(
            source.read_u16(self.ifds.byte_order), "when reading TIFF IFD entry tag"
        );

        // read the entry type
        let entry_type = try_if_eof!(
            source.read_u16(self.ifds.byte_order), "when reading TIFF IFD entry type"
        );

        // read the count
        let count = try_if_eof!(
            source.read_u32(self.ifds.byte_order), "when reading TIFF IFD entry data count"
        );

        // read the offset/value
        let offset = try_if_eof!(
            source.read_u32(self.ifds.byte_order), "when reading TIFF IFD entry data offset"
        );

        println!("---------------------------------");
        println!("Entry tag:                   {:04X}, {}", tag, tag);
        println!("Entry type:                  {:04X}, {}", entry_type, entry_type);
        println!("Entry items count:       {:08X}, {}", count, count);
        println!("Entry data offset/value: {:08X}, {}", offset, offset);
        println!("---------------------------------");

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

/// Designates TIFF IFD entry type, as defined by TIFF spec.
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
    fn size(self) -> Option<u8> {
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
            EntryType::SignedRational => Some(8),
            EntryType::Float          => Some(4),
            EntryType::Double         => Some(8),
            EntryType::Unknown(_)     => None,
        }
    }
}

/// Represents a single TIFF IFD entry.
pub struct Entry<'a, R: Read + Seek + 'a> {
    ifds: &'a LazyIfds<R>,
    tag: u16,
    entry_type: EntryType,
    count: u32,
    offset: u32,
}

impl<'a, R: Read + Seek + 'a> Entry<'a, R> {
    /// Returns the tag of the entry.
    #[inline]
    pub fn tag(&self) -> u16 {
        self.tag
    }

    /// Returns entry type.
    #[inline]
    pub fn entry_type(&self) -> EntryType {
        self.entry_type
    }

    /// Returns the number of items this entry contains.
    #[inline]
    pub fn count(&self) -> u32 {
        self.count
    }

    /// Returns an iterator for elements of the specified representation type.
    ///
    /// This method returns `None` if the requested representation type does not correspond
    /// to the actual type of the entry. Also it returns `None` if the entry type is
    /// unknown.
    #[inline]
    pub fn values<T: EntryTypeRepr>(&self) -> Option<EntryValues<'a, T, R>> {
        // compare the requested repr type with the actual entry type
        if self.entry_type == T::entry_type() {
            // then try to get the size and ignore the data in the entry if it is unknown
            if let Some(entry_type_size) = T::entry_type().size() {
                // if the total entry data size is smaller than 4 bytes (u32 value length)
                // the the data is embedded into the offset u32
                if entry_type_size as u32 * self.count <= 4 {
                    let mut data = [0u8; 4];
                    self.ifds.byte_order.write_u32(&mut data, self.offset);
                    Some(EntryValues::Embedded(EmbeddedValues {
                        current: 0,
                        count: self.count,
                        data: data,
                        byte_order: self.ifds.byte_order,
                        _entry_type_repr: PhantomData,
                    }))
                // othewise the data is stored at that offset
                } else {
                    Some(EntryValues::Referenced(ReferencedValues {
                        ifds: self.ifds,
                        count: self.count,
                        next_offset: self.offset,
                        bytes_read: 0,
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

    /// Returns a vector containing all of the items of this entry, loaded with the specified
    /// representation type.
    ///
    /// This method returns `None` if the requested representation type does not correspond
    /// to the actual type of the entry. Also it returns `None` if the entry type is
    /// unknown.
    #[inline]
    pub fn all_values<T: EntryTypeRepr>(&self) -> Option<Result<Vec<T::Repr>>> {
        // compare the requested repr type with the actual entry type
        if self.entry_type == T::entry_type() {
            // then try to get the size and ignore the data in the entry if it is unknown
            if let Some(entry_type_size) = T::entry_type().size() {
                // if the total entry data size is smaller than 4 bytes (u32 value length)
                // the the data is embedded into the offset u32, and we just delegate to the
                // iterator
                if entry_type_size as u32 * self.count <= 4 {
                    Some(self.values::<T>().unwrap().collect())
                // othewise the data is stored at that offset, load it all at once
                } else {
                    match self.ifds.source.borrow_mut().seek(SeekFrom::Start(self.offset as u64))
                        .map_err(if_eof!(std, "when seeking to the beginning of IFD entry data"))
                    {
                        Ok(_) => {}
                        Err(e) => return Some(Err(e))
                    }

                    let mut result = Vec::new();
                    match T::read_many_from(&mut *self.ifds.source.borrow_mut(),
                                            self.ifds.byte_order, self.count, &mut result)
                        .map_err(if_eof!("when reading TIFF IFD entry values"))
                    {
                        Ok(_) => Some(Ok(result)),
                        Err(e) => Some(Err(e))
                    }
                }

            } else {
                None
            }

        } else {
            None
        }
    }
}

/// Designates a marker type which represent one of TIFF directory entry types.
pub trait EntryTypeRepr {
    /// The represented type, e.g. Rust primitive or a string.
    type Repr;

    /// Returns the entry type corresponding to this marker type.
    fn entry_type() -> EntryType;

    /// Attempts to read the represented value from the given stream with the given byte order.
    ///
    /// Returns the number of bytes read and the value itself.
    fn read_from<R: Read>(source: &mut R, byte_order: ByteOrder) -> byteorder::Result<(u32, Self::Repr)>;

    /// Attempts to read a number of the represented values from the given stream with the given
    /// byte order.
    ///
    /// `n` values will be are stored in `target`, or an error will be returned. `target` vector
    /// may be modified even if this method returns an error.
    fn read_many_from<R: Read>(source: &mut R, byte_order: ByteOrder, n: u32, target: &mut Vec<Self::Repr>) -> byteorder::Result<()>;

    /// Reads the `n`th represented value inside `source`.
    ///
    /// If the value can be read successfully (`n` < `count`, the represented type is smaller
    /// than or equal to u32, etc.), returns `Some(value)`, otherwise returns `None`.
    fn read_from_u32(source: [u8; 4], byte_order: ByteOrder, n: usize, count: usize) -> Option<Self::Repr>;
}

/// Contains representation types for all of defined TIFF entry types.
pub mod entry_types {
    use std::io::Read;
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
                |$u32_source:pat, $u32_byte_order:pat, $n:pat, $count:pat| $u32_read:expr
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
                        // This logic is necessary to handle variable-size items (Ascii strings)
                        // We read item by item, increasing the read bytes counter until we read
                        // all expected items (whose size can be calculated)
                        let item_size = EntryType::$tpe.size().expect("reading unknown data type");
                        let max_bytes = n * item_size as u32;
                        let mut bytes_read = 0;
                        while bytes_read < max_bytes {
                            let (c, v) = try!(Self::read_from(source, byte_order));
                            bytes_read += c;
                            target.push(v);
                        }
                        Ok(())
                    }

                    fn read_from_u32($u32_source: [u8; 4], $u32_byte_order: ByteOrder, $n: usize, $count: usize) -> Option<$repr> {
                        $u32_read
                    }
                }
            )+
        }
    }

    gen_entry_types! {
        Byte, u8,
            |source, _| byteorder::ReadBytesExt::read_u8(source).map(|v| (1, v)),
            |source, _, n, count| if n >= count || n >= 4 { None } else { Some(source[n]) };
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
            |source, _, n, count| if n >= count || n >= 4 { None } else {
                // w x y z
                // +-----0   4
                // 0 +---0   4
                // +---0 0   3, 4
                // 0 +-0 0   3, 4
                // +-0 +-0   2, 4
                // +-0 0 0   2, 3, 4
                // 0 0 +-0   1, 2, 4
                // 0 0 0 0   1, 2, 3, 4
                let bs = source;
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
            |source, byte_order, n, count| if n >= count || n >= 2 { None } else {
                Some(byte_order.read_u16(&source[2*n..]))
            };
        Long, u32,
            |source, byte_order| source.read_u32(byte_order).map(|v| (4, v)),
            |source, byte_order, n, _| if n >= 1 { None } else { Some(byte_order.read_u32(&source)) };
        Rational, (u32, u32),
            |source, byte_order| source.read_u32(byte_order)
                .and_then(|n| source.read_u32(byte_order).map(|d| (n, d)))
                .map(|v| (4 * 2, v)),
            |_, _, _, _| None;
        SignedByte, i8,
            |source, _| byteorder::ReadBytesExt::read_i8(source).map(|v| (1, v)),
            |source, _, n, count| if n >= count || n >= 4 { None } else { Some(source[n] as i8) };
        Undefined, u8,
            |source, _| byteorder::ReadBytesExt::read_u8(source).map(|v| (1, v)),
            |source, _, n, count| if n >= count || n >= 4 { None } else { Some(source[n]) };
        SignedShort, i16,
            |source, byte_order| source.read_i16(byte_order).map(|v| (2, v)),
            |source, byte_order, n, count| if n >= count || n >= 2 { None } else {
                Some(byte_order.read_i16(&source[2*n..]))
            };
        SignedLong, i32,
            |source, byte_order| source.read_i32(byte_order).map(|v| (4, v)),
            |source, byte_order, n, _| if n >= 1 { None } else { Some(byte_order.read_i32(&source)) };
        SignedRational, (i32, i32),
            |source, byte_order| source.read_i32(byte_order)
                .and_then(|n| source.read_i32(byte_order).map(|d| (n, d)))
                .map(|v| (4 * 2, v)),
            |_, _, _, _| None;
        Float, f32,
            |source, byte_order| source.read_f32(byte_order).map(|v| (4, v)),
            |source, byte_order, n, _| if n >= 1 { None } else { Some(byte_order.read_f32(&source)) };
        Double, f64,
            |source, byte_order| source.read_f64(byte_order).map(|v| (8, v)),
            |_, _, _, _| None
    }
}

/// An iterator over values in an TIFF IFD entry.
pub enum EntryValues<'a, T: EntryTypeRepr, R: Read + Seek + 'a> {
    #[doc(hidden)]
    Embedded(EmbeddedValues<T>),
    #[doc(hidden)]
    Referenced(ReferencedValues<'a, T, R>),
}

impl<'a, T: EntryTypeRepr, R: Read + Seek + 'a> Iterator for EntryValues<'a, T, R> {
    type Item = Result<T::Repr>;

    fn next(&mut self) -> Option<Result<T::Repr>> {
        match self.read_value() {
            Ok(result) => result.map(Ok),
            Err(e) => Some(Err(e))
        }
    }
}

impl<'a, T: EntryTypeRepr, R: Read + Seek + 'a> EntryValues<'a, T, R> {
    fn read_value(&mut self) -> Result<Option<T::Repr>> {
        match *self {
            EntryValues::Embedded(ref mut v) => Ok(v.read_value()),
            EntryValues::Referenced(ref mut v) => v.read_value(),
        }
    }
}

#[doc(hidden)]
pub struct EmbeddedValues<T: EntryTypeRepr> {
    current: u32,
    count: u32,
    data: [u8; 4],
    byte_order: ByteOrder,
    _entry_type_repr: PhantomData<T>,
}

impl<T: EntryTypeRepr> EmbeddedValues<T> {
    fn read_value(&mut self) -> Option<T::Repr> {
        if self.current >= self.count {
            None
        } else {
            let result = T::read_from_u32(self.data, self.byte_order, self.current as usize, self.count as usize);
            self.current += 1;
            result
        }
    }
}

#[doc(hidden)]
pub struct ReferencedValues<'a, T: EntryTypeRepr, R: Read + Seek + 'a> {
    ifds: &'a LazyIfds<R>,
    count: u32,
    bytes_read: u32,
    next_offset: u32,
    _entry_type_repr: PhantomData<T>,
}

impl<'a, T: EntryTypeRepr, R: Read + Seek + 'a> ReferencedValues<'a, T, R> {
    fn read_value(&mut self) -> Result<Option<T::Repr>> {
        if self.bytes_read >= self.count * T::entry_type().size().unwrap() as u32 {
            return Ok(None);
        }

        try!(self.ifds.source.borrow_mut().seek(SeekFrom::Start(self.next_offset as u64)));

        let (bytes_read, value) = try_if_eof!(
            T::read_from(&mut *self.ifds.source.borrow_mut(), self.ifds.byte_order),
            "when reading TIFF entry value"
        );
        self.next_offset += bytes_read;
        self.bytes_read += bytes_read;

        Ok(Some(value))
    }
}

#[cfg(test)]
mod tests {
    use std::io::{Write, Cursor};

    use byteorder::{self, ByteOrder, BigEndian, LittleEndian};

    use super::{TiffReader, EntryType, entry_types};

    trait Writable {
        fn write_to<W: Write + ?Sized, T: ByteOrder>(&self, target: &mut W);
    }

    impl Writable for [u8] {
        fn write_to<W: Write + ?Sized, T: ByteOrder>(&self, target: &mut W) {
            target.write_all(self).unwrap();
        }
    }

    impl Writable for i8 {
        fn write_to<W: Write + ?Sized, T: ByteOrder>(&self, target: &mut W) {
            byteorder::WriteBytesExt::write_i8(target, *self).unwrap();
        }
    }

    impl Writable for u8 {
        fn write_to<W: Write + ?Sized, T: ByteOrder>(&self, target: &mut W) {
            byteorder::WriteBytesExt::write_u8(target, *self).unwrap();
        }
    }

    macro_rules! gen_writable {
        ($($t:ty, $f:ident);+) => {
            $(
                impl Writable for $t {
                    fn write_to<W: Write + ?Sized, T: ByteOrder>(&self, target: &mut W) {
                        byteorder::WriteBytesExt::$f::<T>(target, *self).unwrap();
                    }
                }
            )+
        }
    }

    gen_writable! {
        i16, write_i16;
        u16, write_u16;
        i32, write_i32;
        u32, write_u32;
        i64, write_i64;
        u64, write_u64;
        f32, write_f32;
        f64, write_f64
    }

    macro_rules! build {
        ($e:ty, $($arg:expr),+) => {{
            let mut data = Vec::new();
            $($arg.write_to::<_, $e>(&mut data);)+
            data
        }}
    }

    macro_rules! assert_items {
        ($iter:expr $(, $item:expr)*) => {{
            let mut it = $iter;
            $(
            assert_eq!(it.next().unwrap().unwrap(), $item);
            )+
            assert!(it.next().is_none());
        }}
    }

    #[test]
    fn test_big_endian_empty() {
        let data = build! { BigEndian,
            b"MM", 42u16, 0u32
        };

        let reader = TiffReader::new(Cursor::new(data));
        let ifds = reader.ifds().unwrap();
        let mut ifds_iter = (&ifds).into_iter();
        assert!(ifds_iter.next().is_none());
    }

    #[test]
    fn test_little_endian_empty() {
        let data = build! { LittleEndian,
            b"II", 42u16, 0u32
        };

        let reader = TiffReader::new(Cursor::new(data));
        let ifds = reader.ifds().unwrap();
        let mut ifds_iter = (&ifds).into_iter();
        assert!(ifds_iter.next().is_none());
    }

    #[test]
    fn test_one_ifd() {
        let data = build! { BigEndian,
            b"MM", 42u16, 8u32,  // 1st IFD starts from 8th offset
            
            // first IFD
            13u16,

            // first entry, Byte
            4u16, 1u16, 4u32, b"abcd",

            // second entry, Ascii
            8u16, 2u16, 12u32, 170u32,

            // third entry, Short
            15u16, 3u16, 2u32, 23u16, 34u16,

            // fourth entry, Long
            16u16, 4u16, 3u32, 182u32,

            // fifth entry, Rational
            23u16, 5u16, 2u32, 194u32,

            // sixth entry, SignedByte
            42u16, 6u16, 8u32, 210u32,

            // seventh entry, Undefined
            4u16, 7u16, 20u32, 218u32,

            // eighth entry, SignedShort
            8u16, 8u16, 3u32, 238u32,

            // ninth entry, SignedLong
            15u16, 9u16, 1u32, -3724i32,

            // tenth entry, SignedRational
            16u16, 10u16, 1u32, 244u32,

            // eleventh entry, Float
            23u16, 11u16, 1u32, 0.123f32,

            // twelvth entry, Double
            42u16, 12u16, 1u32, 252u32,

            // thirteenth entry, Unknown
            4u16, 123u16, 0u32, 0u32,

            // next IFD offset¸ zero means no more IFDs
            0u32,

            // @170, Ascii, 12 bytes, zero-terminated
            b"hello\x00world\x00",

            // @182, Long x3, 12 bytes,
            123u32, 12u32, 5492957u32,

            // @194, Rational x2, 16 bytes
            22u32, 7u32, 355u32, 113u32,

            // @210, SignedByte x8, 8 bytes
            -3i8, -2i8, -1i8, 0i8, 1i8, 2i8, 3i8, 4i8,

            // @218, Undefined x20, 20 bytes
            "привет мир!".as_bytes(),

            // @238, SignedShort x3, 6 bytes
            -8i16, 0i16, 128i16,

            // @244, SignedRational x1, 8 bytes
            -333i32, -106i32,

            // @252, Double x1, 8 bytes
            3.14f64
        };

        let reader = TiffReader::new(Cursor::new(data));
        
        for ifd in &reader.ifds().unwrap() {
            let ifd = ifd.unwrap();

            assert_eq!(ifd.len(), 13);
            for (i, e) in ifd.enumerate() {
                let e = e.unwrap();

                match i {
                    0 => {
                        assert_eq!(e.tag(), 4);
                        assert_eq!(e.entry_type(), EntryType::Byte);
                        assert_eq!(e.count(), 4);
                        assert_eq!(
                            e.all_values::<entry_types::Byte>().unwrap().unwrap(), 
                            b"abcd".to_owned()
                        );
                        assert_items!(
                            e.values::<entry_types::Byte>().unwrap(),
                            b'a', b'b', b'c', b'd'
                        );
                    }
                    1 => {
                        assert_eq!(e.tag(), 8);
                        assert_eq!(e.entry_type(), EntryType::Ascii);
                        assert_eq!(e.count(), 12);
                        assert_eq!(
                            e.all_values::<entry_types::Ascii>().unwrap().unwrap(), 
                            vec!["hello", "world"]
                        );
                        assert_items!(
                            e.values::<entry_types::Ascii>().unwrap(),
                            "hello".to_owned(), 
                            "world".to_owned()
                        );
                    }
                    2 => {
                        assert_eq!(e.tag(), 15);
                        assert_eq!(e.entry_type(), EntryType::Short);
                        assert_eq!(e.count(), 2);
                        assert_eq!(
                            e.all_values::<entry_types::Short>().unwrap().unwrap(),
                            vec![23, 34]
                        );
                        assert_items!(
                            e.values::<entry_types::Short>().unwrap(),
                            23, 34
                        );
                    }
                    3 => {
                        assert_eq!(e.tag(), 16);
                        assert_eq!(e.entry_type(), EntryType::Long);
                        assert_eq!(e.count(), 3);
                        assert_eq!(
                            e.all_values::<entry_types::Long>().unwrap().unwrap(),
                            vec![123, 12, 5492957]
                        );
                        assert_items!(
                            e.values::<entry_types::Long>().unwrap(),
                            123, 12, 5492957
                        );
                    }
                    4 => {
                        assert_eq!(e.tag(), 23);
                        assert_eq!(e.entry_type(), EntryType::Rational);
                        assert_eq!(e.count(), 2);
                        assert_eq!(
                            e.all_values::<entry_types::Rational>().unwrap().unwrap(),
                            vec![(22, 7), (355, 113)]
                        );
                        assert_items!(
                            e.values::<entry_types::Rational>().unwrap(),
                            (22, 7), (355, 113)
                        )
                    }
                    5 => {
                        assert_eq!(e.tag(), 42);
                        assert_eq!(e.entry_type(), EntryType::SignedByte);
                        assert_eq!(e.count(), 8);
                        assert_eq!(
                            e.all_values::<entry_types::SignedByte>().unwrap().unwrap(),
                            vec![-3, -2, -1, 0, 1, 2, 3, 4]
                        );
                        assert_items!(
                            e.values::<entry_types::SignedByte>().unwrap(),
                            -3, -2, -1, 0, 1, 2, 3, 4
                        );
                    }
                    6 => {
                        assert_eq!(e.tag(), 4);
                        assert_eq!(e.entry_type(), EntryType::Undefined);
                        assert_eq!(e.count(), 20);
                        assert_eq!(
                            e.all_values::<entry_types::Undefined>().unwrap().unwrap(),
                            "привет мир!".to_owned().into_bytes()
                        );
                        assert_items!(
                            e.values::<entry_types::Undefined>().unwrap(),
                            // UTF-8 bytes
                            208, 191, 209, 128, 208, 184, 208, 178, 208, 181, 209, 130, 32,
                            208, 188, 208, 184, 209, 128, 33
                        );
                    }
                    7 => {
                        assert_eq!(e.tag(), 8);
                        assert_eq!(e.entry_type(), EntryType::SignedShort);
                        assert_eq!(e.count(), 3);
                        assert_eq!(
                            e.all_values::<entry_types::SignedShort>().unwrap().unwrap(),
                            vec![-8, 0, 128]
                        );
                        assert_items!(
                            e.values::<entry_types::SignedShort>().unwrap(),
                            -8, 0, 128
                        );
                    }
                    8 => {
                        assert_eq!(e.tag(), 15);
                        assert_eq!(e.entry_type(), EntryType::SignedLong);
                        assert_eq!(e.count(), 1);
                        assert_eq!(
                            e.all_values::<entry_types::SignedLong>().unwrap().unwrap(),
                            vec![-3724]
                        );
                        assert_items!(
                            e.values::<entry_types::SignedLong>().unwrap(),
                            -3724
                        );
                    }
                    9 => {
                        assert_eq!(e.tag(), 16);
                        assert_eq!(e.entry_type(), EntryType::SignedRational);
                        assert_eq!(e.count(), 1);
                        assert_eq!(
                            e.all_values::<entry_types::SignedRational>().unwrap().unwrap(),
                            vec![(-333, -106)]
                        );
                        assert_items!(
                            e.values::<entry_types::SignedRational>().unwrap(),
                            (-333, -106)
                        );
                    }
                    10 => {
                        assert_eq!(e.tag(), 23);
                        assert_eq!(e.entry_type(), EntryType::Float);
                        assert_eq!(e.count(), 1);
                        assert_eq!(
                            e.all_values::<entry_types::Float>().unwrap().unwrap(),
                            vec![0.123]
                        );
                        assert_items!(
                            e.values::<entry_types::Float>().unwrap(),
                            0.123
                        );
                    }
                    11 => {
                        assert_eq!(e.tag(), 42);
                        assert_eq!(e.entry_type(), EntryType::Double);
                        assert_eq!(e.count(), 1);
                        assert_eq!(
                            e.all_values::<entry_types::Double>().unwrap().unwrap(),
                            vec![3.14]
                        );
                        assert_items!(
                            e.values::<entry_types::Double>().unwrap(),
                            3.14
                        );
                    }
                    12 => {
                        assert_eq!(e.tag(), 4);
                        assert_eq!(e.entry_type(), EntryType::Unknown(123));
                        assert_eq!(e.count(), 0);
                    }
                    _ => {
                        panic!("Too many IFD entries");
                    }
                }
            }
        }
    }

    // one IFD
    // two IFDs
    // first - third - second IFDs
    // reading IFD entries
    //   all types
    //   all embeddable types
}
