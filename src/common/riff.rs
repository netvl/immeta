use std::io::{self, Read, Take};
use std::str;
use std::result;

use byteorder::{ReadBytesExt, LittleEndian};

use types::Result;
use utils::ReadExt;

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
pub struct ChunkId([u8; 4]);

impl ChunkId {
    #[inline]
    pub fn as_str(&self) -> Option<&str> {
        str::from_utf8(&self.0).ok()
    }

    #[inline]
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

pub struct RiffReader<R: Read> {
    source: R
}

impl<R: Read> RiffReader<R> {
    pub fn new(source: R) -> RiffReader<R> {
        RiffReader {
            source: source
        }
    }

    pub fn root(&mut self) -> Result<RiffChunk> {
        let (id, len) = match try!(read_id_and_len(&mut self.source)) {
            Some(t) => t,
            None => return Err(unexpected_eof!())
        };

        if id.as_bytes() != b"RIFF" {
            return Err(invalid_format!("RIFF file header is invalid"));
        }

        Ok(RiffChunk {
            data: Counter {
                delegate: (&mut self.source as &mut Read).take(len as u64),
                counter: None
            },
            tainted: false,
            chunk_id: id,
            len: len
        })
    }
}

struct Counter<'a, R> {
    delegate: R,
    counter: Option<&'a mut u32>
}

impl<'a, R: Read> Read for Counter<'a, R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.delegate.read(buf).map(|n| {
            if let Some(ref mut counter) = self.counter {
                **counter += n as u32;
            }
            n
        })
    }
}

pub struct RiffChunk<'a> {
    chunk_id: ChunkId,
    len: u32,
    tainted: bool,
    data: Counter<'a, Take<&'a mut Read>>
}

impl<'a> RiffChunk<'a> {
    #[inline]
    pub fn chunk_id(&self) -> ChunkId { self.chunk_id }

    #[inline]
    pub fn len(&self) -> u32 { self.len }

    #[inline]
    pub fn contents(&mut self) -> &mut Read { 
        self.tainted = true;
        &mut self.data
    }

    #[inline]
    pub fn can_have_subchunks(&self) -> bool {
        !self.tainted && match &self.chunk_id.0 {
            b"RIFF" | b"LIST" => true,
            _ => false
        }
    }

    #[inline]
    pub fn into_list(self) -> result::Result<Result<RiffListChunk<'a>>, RiffChunk<'a>> {
        if self.can_have_subchunks() {
            Ok(self.into_list_unchecked())
        } else {
            Err(self)
        }
    }

    fn into_list_unchecked(mut self) -> Result<RiffListChunk<'a>> {
        let mut chunk_type = [0u8; 4];

        if try!(self.data.read_exact(&mut chunk_type)) != 4 {
            return Err(unexpected_eof!());
        }

        Ok(RiffListChunk {
            chunk_id: self.chunk_id,
            len: self.len,
            chunk_type: ChunkId(chunk_type),
            data: self.data,
            cur_chunk_len: 0,
            cur_chunk_read: 0
        })
    }
}

pub struct RiffListChunk<'a> {
    chunk_id: ChunkId,
    len: u32,
    chunk_type: ChunkId,
    data: Counter<'a, Take<&'a mut Read>>,
    cur_chunk_len: u32,
    cur_chunk_read: u32
}

impl<'a> RiffListChunk<'a> {
    #[inline]
    pub fn chunk_id(&self) -> ChunkId { self.chunk_id }

    #[inline]
    pub fn len(&self) -> u32 { self.len }

    #[inline]
    pub fn chunk_type(&self) -> ChunkId { self.chunk_type }

    #[inline]
    pub fn next(&mut self) -> Option<Result<RiffChunk>> {
        if self.cur_chunk_read < self.cur_chunk_len {
            let to_skip = (self.cur_chunk_len - self.cur_chunk_read) as u64;
            match self.data.skip_exact_0(to_skip) {
                Ok(n) if n == to_skip => {}
                Ok(_) => return Some(Err(unexpected_eof!())),
                Err(e) => return Some(Err(e.into()))
            }
        }

        let (id, len) = match read_id_and_len(&mut self.data) {
            Ok(Some(t)) => t,
            Ok(None) => return None,
            Err(e) => return Some(Err(e.into()))
        };

        self.cur_chunk_read = 0;
        self.cur_chunk_len = len;

        Some(Ok(RiffChunk {
            chunk_id: id,
            len: len,
            tainted: false,
            data: Counter {
                delegate: (&mut self.data as &mut Read).take(len as u64),
                counter: Some(&mut self.cur_chunk_read)
            }
        }))
    }
}

fn read_id_and_len<R: Read>(source: &mut R) -> Result<Option<(ChunkId, u32)>> {
    let mut id = [0u8; 4];

    match try!(source.read_exact(&mut id)) {
        0 => return Ok(None),
        4 => {}
        _ => return Err(unexpected_eof!())
    }

    let len = try!(source.read_u32::<LittleEndian>());

    Ok(Some((ChunkId(id), len)))
}

#[cfg(test)]
mod tests {
    use std::io::{Read, Write};

    use byteorder::{WriteBytesExt, LittleEndian};

    use utils::ReadExt;

    use super::{RiffReader, RiffListChunk, ChunkId};

    macro_rules! build {
        ($($arg:expr),+) => {{
            let mut data = Vec::new();
            $(data.write($arg).unwrap();)+
            data
        }}
    }

    fn n(n: u32) -> [u8; 4] {
        let mut r = [0u8; 4];
        (&mut r as &mut [u8]).write_u32::<LittleEndian>(n).unwrap();
        r
    }

    fn check_next_chunk<'a>(cl: &mut RiffListChunk<'a>, id: ChunkId, len: u32, data: &[u8]) {
        let chunk = cl.next();
        let mut chunk = chunk.unwrap().unwrap();

        assert_eq!(chunk.chunk_id(), id);
        assert_eq!(chunk.len(), len);
        let contents = chunk.contents().read_to_vec().unwrap();
        assert_eq!(&*contents, data);
        assert!(!chunk.can_have_subchunks());
    }

    #[test]
    fn test_invalid_header() {
        let mut data = b"XXXX\x04abcd" as &[u8];

        let mut r = RiffReader::new(&mut data);

        let root = r.root();
        assert!(root.is_err());
    }

    #[test]
    fn test_flat_chunks() {
        let data = build! {
            b"RIFF", &n(37), b"abcd",
            b"A   ", &n(4), b"1234",
            b"B   ", &n(5), b"56789"
        };
        let mut data: &[u8] = &data;

        let mut r = RiffReader::new(&mut data);

        let mut root = r.root().unwrap();

        assert_eq!(root.chunk_id(), ChunkId(*b"RIFF"));
        assert_eq!(root.len(), 37);
        assert!(root.can_have_subchunks());

        let root = root.into_list();
        assert!(root.is_ok());
        let mut root = root.ok().unwrap().unwrap();

        assert_eq!(root.chunk_type(), ChunkId(*b"abcd"));

        check_next_chunk(&mut root, ChunkId(*b"A   "), 4, b"1234");
        check_next_chunk(&mut root, ChunkId(*b"B   "), 5, b"56789");

        assert!(root.next().is_none());
    }

    #[test]
    fn test_nested_chunks() {
        let data = build! {
            b"RIFF", &n(77), b"abcd",
            b"A   ", &n(1), b"z",
            b"LIST", &n(56), b"wxyz",
                b" B  ", &n(3), b"123",
                b"LIST", &n(22), b"hi  ",
                    b"  C ", &n(0),
                    b"   D", &n(2), b"op",
                b"E   ", &n(3), b"fuz"
        };
        let mut data: &[u8] = &data;

        let mut r = RiffReader::new(&mut data);

        let mut root = r.root().unwrap();

        assert_eq!(root.chunk_id(), ChunkId(*b"RIFF"));
        assert_eq!(root.len(), 77);
        assert!(root.can_have_subchunks());

        let root = root.into_list();
        assert!(root.is_ok());
        let mut root = root.ok().unwrap().unwrap();

        assert_eq!(root.chunk_type(), ChunkId(*b"abcd"));

        check_next_chunk(&mut root, ChunkId(*b"A   "), 1, b"z");

        {
            let chunk = root.next().unwrap().unwrap();
            assert_eq!(chunk.chunk_id(), ChunkId(*b"LIST"));
            assert_eq!(chunk.len(), 56);
            assert!(chunk.can_have_subchunks());

            let chunk = chunk.into_list();
            assert!(chunk.is_ok());
            let mut chunk = chunk.ok().unwrap().unwrap();

            check_next_chunk(&mut chunk, ChunkId(*b" B  "), 3, b"123");

            {
                let sublist = chunk.next().unwrap().unwrap();
                assert_eq!(sublist.chunk_id(), ChunkId(*b"LIST"));
                assert_eq!(sublist.len(), 22);
                assert!(sublist.can_have_subchunks());

                let sublist = sublist.into_list();
                assert!(sublist.is_ok());
                let mut sublist = sublist.ok().unwrap().unwrap();

                check_next_chunk(&mut sublist, ChunkId(*b"  C "), 0, b"");
                check_next_chunk(&mut sublist, ChunkId(*b"   D"), 2, b"op");
            }

            check_next_chunk(&mut chunk, ChunkId(*b"E   "), 3, b"fuz");

            assert!(chunk.next().is_none());
        }

        assert!(root.next().is_none());
    }

    #[test]
    fn test_skip_chunk_data() {
        let data = build! {
            b"RIFF", &n(77), b"abcd",
            b"A   ", &n(10), b"abcdefghij",
            b" B  ", &n(12), b"123456789012",
            b"  C ", &n(8),  b"ABCDEFGH"
        };
        let mut data: &[u8] = &data;

        let mut r = RiffReader::new(&mut data);

        let root = r.root().unwrap();
        let mut root = root.into_list().ok().unwrap().unwrap();

        {
            let mut chunk = root.next().unwrap().unwrap();
            assert_eq!(chunk.chunk_id(), ChunkId(*b"A   "));
            assert_eq!(chunk.len(), 10);
            assert_eq!(
                (&mut chunk.contents() as &mut Read).take(5).read_to_vec().unwrap(),
                b"abcde".to_owned()
            );
        }

        {
            let chunk = root.next().unwrap().unwrap();
            assert_eq!(chunk.chunk_id(), ChunkId(*b" B  "));
            assert_eq!(chunk.len(), 12);
        }

        {
            let mut chunk = root.next().unwrap().unwrap();
            assert_eq!(chunk.chunk_id(), ChunkId(*b"  C "));
            assert_eq!(chunk.len(), 8);
            assert_eq!(chunk.contents().read_to_vec().unwrap(), b"ABCDEFGH".to_owned());
        }

        assert!(root.next().is_none());
    }
}
