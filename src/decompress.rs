use crate::Buf;
use core::fmt;

macro_rules! read_u8 {
    ($arr:ident) => {
        $arr.next().copied().ok_or(Error::UnexpectedEof)?
    };
}

macro_rules! read_int {
    ($arr:ident, $first:expr) => {{
        let first = $first;
        if first == 15 {
            let x = $arr
                .take_while(|x| **x == 255)
                .map(|x| *x as usize)
                .sum::<usize>()
                + (read_u8!($arr) as usize);
            first + x
        } else {
            first
        }
    }};
}

/// The error type that is returned by [`compression`](crate::compression) or [`decompression`](crate::decompression).
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub enum Error {
    /// Inidicates that the `out` pointer didn't contain enough memory
    /// to store the de-/compressed result.
    MemoryLimitExceeded,
    /// Tried to read more bytes, but there were no bytes left in the given data.
    UnexpectedEof,
    /// The offset for duplicating data is 0, but it 0 is an invalid value and should never
    /// be used as the offset.
    ///
    /// This is most likely caused by trying to decompress invalid input.
    ZeroMatchOffset,
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Error::MemoryLimitExceeded => f.write_str("not enough memory available in out pointer"),
            Error::UnexpectedEof => {
                f.write_str("expected at lelast one byte to be read, but instead end was reached")
            }
            Error::ZeroMatchOffset => f.write_str(
                "The offset was zero. This is most likely caused by trying to parse invalid input.",
            ),
        }
    }
}

pub fn decompress<S: Buf<u8>>(data: &[u8], out: &mut S) -> Result<(), Error> {
    let mut reader = data.iter();
    let reader = reader.by_ref();

    // loop through all sequences
    while let Some(&token) = reader.next() {
        // the first part of a sequence is the token.
        // the token is composed of two 4-bit-wide bitfields.
        // the first one describes the length of the literal, if one or more is present.
        //
        // if the len is 15, there are more bytes that describe the length
        let len = (token >> 4) as usize;
        let len = read_int!(reader, len);

        // now copy `len` literal bytes into the output
        if !out.reserve(len) {
            return Err(Error::MemoryLimitExceeded);
        }
        out.extend(len, reader.take(len).copied());

        // read low byte of the next offset
        let low = match reader.next() {
            Some(&low) => low,
            // this is the last sequence, because there is no
            // data left that has to be duplicated
            None => break,
        };

        // read offset for the duplicated data
        let offset = u16::from_le_bytes([low, read_u8!(reader)]);

        // the match length represents the number we copy the data.
        // it's stored in the second bitfield of the token.
        //
        // the minimum value of the len is 4, which leads to 19 as the maxium value
        let len = 4 + read_int!(reader, (token & 0xF) as usize);

        // now copy the data that is duplicated
        copy(offset as usize, len, out)?;
    }

    Ok(())
}

/// Optimized version of the copy operation.
fn copy<S: Buf<u8>>(offset: usize, len: usize, out: &mut S) -> Result<(), Error> {
    let out_len = out.len();

    match offset {
        // invalid offset
        0 => return Err(Error::ZeroMatchOffset),
        // repeat the last byte we output
        1 => {
            if !out.resize(
                out_len + len,
                out.as_slice()
                    .last()
                    .copied()
                    .expect("output should ever be filled here"),
            ) {
                return Err(Error::MemoryLimitExceeded);
            }
        }
        // copy each byte manually
        offset => {
            if !out.reserve(len) {
                return Err(Error::MemoryLimitExceeded);
            }
            let start = out_len - offset;
            (0..len).for_each(|idx| {
                let x = out.as_slice()[start + idx];
                out.push(x);
            });
        }
    };

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::{ArrayBuf, Buf};

    fn decompress<'res, S: Buf<u8>>(buf: &'res mut S, input: &[u8]) -> &'res str {
        super::decompress(input, buf).unwrap();
        core::str::from_utf8(buf.as_slice()).unwrap()
    }

    #[test]
    fn empty() {
        let mut buf = ArrayBuf::<u8, 0>::new();
        assert_eq!(decompress(&mut buf, &[]), "");
    }

    #[test]
    fn hello() {
        let raw = [0x11, b'a', 1, 0];
        let mut buf = ArrayBuf::<u8, 6>::new();
        assert_eq!(decompress(&mut buf, &raw), "aaaaaa");
    }

    #[test]
    fn more() {
        let raw = "8B1UaGUgcXVpY2sgYnJvd24gZm94IGp1bXBzIG92ZXIgdGhlIGxhenkgZG9nLg==";
        let raw = base64::decode(raw).unwrap();

        let mut buf = ArrayBuf::<u8, 128>::new();
        assert_eq!(
            decompress(&mut buf, &raw),
            "he quick brown fox jumps over the lazy dog."
        );
    }
}
