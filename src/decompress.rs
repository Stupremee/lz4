//! Implementation of decompression of lz4 compressed data.

use crate::Buf;
use core::fmt;

macro_rules! read_int {
    ($arr:ident, $first:expr) => {{
        let first = $first;
        if first == 15 {
            let x = $arr
                .take_while(|x| **x == 255)
                .map(|x| *x as usize)
                .sum::<usize>()
                + (read_byte($arr.by_ref())? as usize);
            first + x
        } else {
            first
        }
    }};
}

/// The magic number which is at the start of every
/// compressed data in the frame format.
const MAGIC: u32 = 0x184D2204;

/// The error type that is returned by various decompression-related methods.
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

    /// The data that was tried to decompress, started with an invalid magic number.
    ///
    /// This is most likely caused by trying to decompress invalid input.
    InvalidMagic,
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

            Error::InvalidMagic => f.write_str(
                "The magic number is invalid. This is most likely caused by trying to parse invalid input.",
            ),
        }
    }
}

#[inline]
fn read<'byte, I: Iterator<Item = &'byte u8>, const N: usize>(reader: I) -> Result<[u8; N], Error> {
    let mut buf = [0u8; N];

    let mut count = 0;
    for (entry, val) in buf.iter_mut().zip(reader.take(N).copied()) {
        *entry = val;
        count += 1;
    }

    if count != N {
        Err(Error::UnexpectedEof)
    } else {
        Ok(buf)
    }
}

#[inline]
fn read_byte<'byte, I: Iterator<Item = &'byte u8>>(reader: I) -> Result<u8, Error> {
    Ok(read::<I, 1>(reader)?[0])
}

/// Decompressed LZ4-compressed data and pushes the decompressed data into the
/// output buf.
///
/// This function is capable of parsing and decompressing
/// data that was compressed using the [Frame format] described
/// by the LZ4 specification.
///
/// [Frame format]: https://github.com/lz4/lz4/blob/dev/doc/lz4_Frame_format.md
pub fn decompress<O: Buf<u8>>(input: &[u8], output: &mut O) -> Result<(), Error> {
    let mut reader = input.iter();
    let reader = reader.by_ref();

    let magic = u32::from_le_bytes(read(reader)?);

    if magic != MAGIC {
        return Err(Error::InvalidMagic);
    }
    todo!()
}

/// Decompresses a LZ4-compressed block of `data`
///
/// The decompressed data will be written into the `out` buffer. If the buffer
/// doesn't have enough memory, an error will be returned.
///
/// Note that this method is not able to decompress data that was compressed
/// by tools like [`lz4`](https://lz4.github.io/lz4) since the compressed data
/// is compressed using the frame format. For decompressing data like this use
/// [`decompress`](crate::decompress::decompress) function instead.
pub fn decompress_block<O: Buf<u8>>(data: &[u8], out: &mut O) -> Result<(), Error> {
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
        let offset = u16::from_le_bytes([low, read_byte(reader.by_ref())?]);

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

// TODO: Probably replace with `ptr::copy_nonoverlapping`
/// Optimized version of the copy operation.
fn copy<O: Buf<u8>>(offset: usize, len: usize, out: &mut O) -> Result<(), Error> {
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

    fn decompress_block<'res, S: Buf<u8>>(buf: &'res mut S, input: &[u8]) -> &'res str {
        super::decompress_block(input, buf).unwrap();
        core::str::from_utf8(buf.as_slice()).unwrap()
    }

    #[test]
    fn block_empty() {
        let mut buf = ArrayBuf::<u8, 0>::new();
        assert_eq!(decompress_block(&mut buf, &[]), "");
    }

    #[test]
    fn block_hello() {
        let raw = [0x11, b'a', 1, 0];
        let mut buf = ArrayBuf::<u8, 6>::new();
        assert_eq!(decompress_block(&mut buf, &raw), "aaaaaa");
    }

    #[test]
    fn block_more() {
        let raw = "8B1UaGUgcXVpY2sgYnJvd24gZm94IGp1bXBzIG92ZXIgdGhlIGxhenkgZG9nLg==";
        let raw = base64::decode(raw).unwrap();

        let mut buf = ArrayBuf::<u8, 128>::new();
        assert_eq!(
            decompress_block(&mut buf, &raw),
            "he quick brown fox jumps over the lazy dog."
        );
    }

    #[test]
    fn hello() {
        let raw = "BCJNGGRApwYAAIBoZWxsbwoAAAAA+VtrlA==";
        let raw = base64::decode(raw).unwrap();

        let mut buf = ArrayBuf::<u8, 5>::new();
        super::decompress(&raw, &mut buf).unwrap();
        assert_eq!(core::str::from_utf8(buf.as_slice()), Ok("hello"));
    }
}
