//! Implementation of decompressing raw LZ4-blocks.

use super::{ByteIter, DecompressError};
use crate::Buf;

/// Decompresses a LZ4-compressed block of `data`
///
/// The decompressed data will be written into the `out` buffer. If the buffer
/// doesn't have enough memory, an error will be returned.
///
/// Note that this method is not able to decompress data that was compressed
/// by tools like [`lz4`](https://lz4.github.io/lz4) since the compressed data
/// is compressed using the frame format. For decompressing data like this use
/// [`decompress`](crate::decompress::decompress) function instead.
pub fn decompress_block<O: Buf<u8>>(data: &[u8], out: &mut O) -> Result<(), DecompressError> {
    let mut reader = ByteIter::new(data);

    // loop through all sequences
    while let Ok(token) = reader.read_byte() {
        // the first part of a sequence is the token.
        // the token is composed of two 4-bit-wide bitfields.
        // the first one describes the length of the literal, if one or more is present.
        //
        // if the len is 15, there are more bytes that describe the length
        let len = reader.read_int((token >> 4) as usize)?;

        // now copy `len` literal bytes into the output
        if !out.reserve(len) {
            return Err(DecompressError::MemoryLimitExceeded);
        }
        let slice = reader.take(len)?;
        out.extend(slice);

        // read low byte of the next offset
        let low = match reader.read_byte() {
            Ok(low) => low,
            // this is the last sequence, because there is no
            // data left that has to be duplicated
            Err(_) => break,
        };

        // read offset for the duplicated data
        let offset = u16::from_le_bytes([low, reader.read_byte()?]);

        // the match length represents the number we copy the data.
        // it's stored in the second bitfield of the token.
        //
        // the minimum value of the len is 4, which leads to 19 as the maxium value
        let len = 4 + reader.read_int((token & 0xF) as usize)?;

        // now copy the data that is duplicated
        copy(offset as usize, len, out)?;
    }

    Ok(())
}

// TODO: Probably replace with `ptr::copy`
/// Optimized version of the copy operation.
fn copy<O: Buf<u8>>(offset: usize, len: usize, out: &mut O) -> Result<(), DecompressError> {
    let out_len = out.len();

    match offset {
        // invalid offset
        0 => return Err(DecompressError::ZeroMatchOffset),
        // repeat the last byte we output
        1 => {
            if !out.resize(
                out_len + len,
                out.as_slice()
                    .last()
                    .copied()
                    .expect("output should ever be filled here"),
            ) {
                return Err(DecompressError::MemoryLimitExceeded);
            }
        }
        // copy each byte manually
        offset => {
            if !out.reserve(len) {
                return Err(DecompressError::MemoryLimitExceeded);
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
            "The quick brown fox jumps over the lazy dog."
        );
    }
}
