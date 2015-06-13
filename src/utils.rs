use std::io::{self, Read, BufRead, ErrorKind};

pub trait ReadExt: Read {
    fn read_exact(&mut self, mut buf: &mut [u8]) -> io::Result<bool> {
        let orig_len = buf.len() as u64;
        io::copy(&mut self.take(orig_len), &mut buf).map(|n| n == orig_len)
    }
}

impl<R: ?Sized + Read> ReadExt for R {}

pub trait BufReadExt: BufRead {
    fn drop_exact(&mut self, n: u64) -> io::Result<u64> {
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

    fn drop_until(&mut self, delim: u8) -> io::Result<usize> {
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
