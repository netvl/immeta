use std::io::{BufRead, Seek, SeekFrom};

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

    pub fn ifds(&mut self) -> Result<Ifds<R>> {
        let mut bom = [0u8; 2];
        try_if_eof!(std, self.source.read_exact(&mut bom), "while reading byte order mark");

        let byte_order = match &bom {
            b"II" => ByteOrder::Little,
            b"MM" => ByteOrder::Big,
            _ => return Err(invalid_format!("invalid TIFF BOM: {:?}", bom))
        };

        let magic = try_if_eof!(
            self.source.read_u16(byte_order), 
            "when reading magic number"
        );
        if magic != 42 {
            return Err(invalid_format!("invalid TIFF magic number: {}", magic));
        }

        Ok(Ifds {
            source: &mut self.source, 
            byte_order: byte_order,
            next_ifd_offset: 4,
        })
    }
}

pub struct Ifds<'a, R: BufRead + Seek + 'a> {
    source: &'a mut R,
    byte_order: ByteOrder,
    next_ifd_offset: u32,
}

impl<'a, R: BufRead + Seek> Iterator for Ifds<'a, R> {
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
        if self.next_ifd_offset == 0 {
            return Ok(None);
        }

        let next_ifd_size = try_if_eof!(
            self.source.read_u16(self.byte_order), "when reading number of entries in an IFD"
        ) as u32;
        if next_ifd_size == 0 {
            return Err(invalid_format!("number of entries in an IFD is zero"));
        }

        // compute the offset of the next IFD offset
        let next_ifd_offset_offset = self.next_ifd_offset + 2 + next_ifd_size * 12;

        try_if_eof!(std,
            self.source.seek(SeekFrom::Start(next_ifd_offset_offset as u64)),
            "when seeking to the next IFD offset"
        );

        let current_ifd_offset = self.next_ifd_offset;
        self.next_ifd_offset = try_if_eof!(
            self.source.read_u16(self.byte_order), "when reading the next IFD offset"
        ) as u32;

        try!(self.source.seek(SeekFrom::Start(current_ifd_offset as u64)));

        // now we're at the first IFD entry
        Ok(Some(Ifd(self.source, self.byte_order)))
    }
}

pub struct Ifd<'a, R: BufRead + Seek + 'a>(&'a mut R, ByteOrder);
