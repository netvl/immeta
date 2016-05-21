use std::io::{self, Read, BufRead, ErrorKind};

use byteorder::{ReadBytesExt, LittleEndian, BigEndian};
use byteorder::ByteOrder as ByteOrderTrait;

pub trait ReadExt: Read {
    fn read_exact_0(&mut self, mut buf: &mut [u8]) -> io::Result<usize> {
        let orig_len = buf.len() as u64;
        io::copy(&mut self.take(orig_len), &mut buf).map(|r| r as usize)
    }

    fn skip_exact_0(&mut self, n: u64) -> io::Result<u64> {
        io::copy(&mut self.take(n), &mut io::sink())
    }

    fn read_to_vec(&mut self) -> io::Result<Vec<u8>> {
        let mut buf = Vec::new();
        try!(self.read_to_end(&mut buf));
        Ok(buf)
    }
}

impl<R: ?Sized + Read> ReadExt for R {}

pub trait BufReadExt: BufRead {
    fn skip_exact(&mut self, n: u64) -> io::Result<u64> {
        let mut skipped = 0;
        loop {
            let available = match self.fill_buf() {
                Ok(n) => n.len(),
                Err(ref e) if e.kind() == ErrorKind::Interrupted => continue,
                Err(e) => return Err(e)
            } as u64;
            let total = skipped + available;
            if total >= n {
                let extra = total - n;
                let to_skip = available - extra;
                skipped += to_skip;
                self.consume(to_skip as usize);
                break;
            }
            self.consume(available as usize);
            skipped += available;
            if available == 0 {
                break;
            }
        }
        Ok(skipped)
    }

    fn skip_until(&mut self, delim: u8) -> io::Result<usize> {
        let mut read = 0;
        loop {
            let (done, used) = {
                let available = match self.fill_buf() {
                    Ok(n) => n,
                    Err(ref e) if e.kind() == ErrorKind::Interrupted => continue,
                    Err(e) => return Err(e)
                };
                match available.iter().position(|&b| b == delim) {
                    Some(i) => (true, i + 1),
                    None => (false, available.len()),
                }
            };
            self.consume(used);
            read += used;
            if done || used == 0 {
                return Ok(read);
            }
        }
    }
}

impl<R: ?Sized + BufRead> BufReadExt for R {}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum ByteOrder {
    Little,
    Big,
}

macro_rules! gen_byte_order_ops {
    ($($read_name:ident, $write_name:ident -> $tpe:ty),+) => {
        impl ByteOrder {
            $(
            #[inline]
            pub fn $read_name(self, source: &[u8]) -> $tpe {
                match self {
                    ByteOrder::Little => LittleEndian::$read_name(source),
                    ByteOrder::Big => BigEndian::$read_name(source),
                }
            }

            pub fn $write_name(self, target: &mut [u8], n: $tpe) {
                match self {
                    ByteOrder::Little => LittleEndian::$write_name(target, n),
                    ByteOrder::Big => BigEndian::$write_name(target, n),
                }
            }
            )+
        }
    }
}

gen_byte_order_ops! {
    read_u16, write_u16 -> u16,
    read_u32, write_u32 -> u32,
    read_u64, write_u64 -> u64,
    read_i16, write_i16 -> i16,
    read_i32, write_i32 -> i32,
    read_i64, write_i64 -> i64,
    read_f32, write_f32 -> f32,
    read_f64, write_f64 -> f64
}


macro_rules! gen_read_byte_order_ext {
    ($tr:ident, $($name:ident -> $tpe:ty),+) => {
        pub trait $tr: Read {
            $(
            #[inline]
            fn $name(&mut self, byte_order: ByteOrder) -> io::Result<$tpe> {
                match byte_order {
                    ByteOrder::Little => ReadBytesExt::$name::<LittleEndian>(self),
                    ByteOrder::Big => ReadBytesExt::$name::<BigEndian>(self),
                }
            }
            )+
        }
    }
}

gen_read_byte_order_ext! {
    ByteOrderReadExt,
    read_u16 -> u16,
    read_u32 -> u32,
    read_u64 -> u64,
    read_i16 -> i16,
    read_i32 -> i32,
    read_i64 -> i64,
    read_f32 -> f32,
    read_f64 -> f64
}

impl<R: Read> ByteOrderReadExt for R {}
