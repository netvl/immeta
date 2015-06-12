use std::io::{self, BufRead, ErrorKind};

pub fn drop_bytes<R: BufRead + ?Sized>(r: &mut R, n: u64) -> io::Result<u64> {
    let mut skipped = 0;
    loop {
        let available = match r.fill_buf() {
            Ok(n) => n.len(),
            Err(ref e) if e.kind() == ErrorKind::Interrupted => continue,
            Err(e) => return Err(e)
        } as u64;
        let total = skipped + available;
        if total >= n {
            let extra = total - n;
            let to_skip = available - extra;
            skipped += to_skip;
            r.consume(to_skip as usize);
            break;
        }
        r.consume(available as usize);
        skipped += available;
        if available == 0 {
            break;
        }
    }
    Ok(skipped)
}

pub fn drop_until<R: BufRead + ?Sized>(r: &mut R, delim: u8) -> io::Result<usize> {
    let mut read = 0;
    loop {
        let (done, used) = {
            let available = match r.fill_buf() {
                Ok(n) => n,
                Err(ref e) if e.kind() == ErrorKind::Interrupted => continue,
                Err(e) => return Err(e)
            };
            match available.iter().cloned().position(|b| b == delim) {
                Some(i) => (true, i + 1),
                None => (false, available.len()),
            }
        };
        r.consume(used);
        read += used;
        if done || used == 0 {
            return Ok(read);
        }
    }
}
