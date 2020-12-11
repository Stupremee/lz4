//! Implementation of decompression of lz4 compressed data.

use crate::Buf;
use core::fmt;

mod iter;
pub(crate) use iter::ByteIter;

pub mod raw;

/// The magic number which is at the start of every
/// compressed data in the frame format.
const MAGIC: u32 = 0x184D2204;

/// The version this decompresser is capable of decompressing.
const VERSION: u8 = 0b01;

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
    /// Tried to decompress a version of LZ4 that is currently not supported.
    ///
    /// This can be caused by either providing an invalid input
    /// or using another version of the specification.
    VersionNotSupported,
    /// Invalid input provided.
    InvalidInput,
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
            Error::VersionNotSupported => f.write_str("The data was comrpessed using a version of LZ4 that is not supported."),
            Error::InvalidInput => f.write_str("The provided data is invalid."),
        }
    }
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
    //let mut reader = input.iter();

    //// every LZ4-frame-format data starts with a magic number
    //let magic = u32::from_le_bytes(read(reader.by_ref())?);
    //if magic != MAGIC {
    //return Err(Error::InvalidMagic);
    //}

    //// after the magic comes the frame descriptor, which
    //// contains several flags and other data that may be important.
    //let flag = read_byte(reader.by_ref())?;

    //// high 2 bits are the version of this data
    //let version = flag >> 6;
    //if version != VERSION {
    //return Err(Error::VersionNotSupported);
    //}

    //// currently not needed. only required once we want to decompress
    //// in parallel.
    //let _block_independence = flag & 0x20;

    //// if 1, each block contains a checksum to validate
    //// the data and detect corruption.
    //let block_checksum = (flag & 0x10) != 0;
    //assert!(!block_checksum, "Checksums are currently not supported");

    //// if 1, the size of the whole compressed data will be encoded
    //// in the frame header
    //let content_size = (flag & 0x08) != 0;

    //// if 1, there will be another checksum at the end of the data,
    //// to detect data corruption.
    //let content_checksum = (flag & 0x04) != 0;
    //assert!(!content_checksum, "Checksums are currently not supported");

    //// if 1, there will be 4 byte wide dictionary id
    //// in the frame header.
    //let dictionary = (flag & 0x01) != 0;

    //// the next byte is the block descriptor.
    ////
    //// currently it only contains the maximum size
    //// of the original (uncompressed) of a block.
    //let bd = read_byte(reader.by_ref())?;

    //// the value inside the 3 bits of the block descriptor
    //// can be converted to an actual size using the following table
    ////
    //// currently not required
    //let size_idx = (bd >> 4) & 0x7;
    //let _size = match size_idx {
    //// 64KB
    //4 => 64 << 10,
    //// 256KB
    //5 => 256 << 10,
    //// 1MB
    //6 => 1 << 20,
    //// 4MB
    //7 => 4 << 20,
    //_ => return Err(Error::InvalidInput),
    //};

    //// read the actual content size if the flag was present
    //let content_size = if content_size {
    //let num = u64::from_le_bytes(read(reader.by_ref())?);
    //Some(num)
    //} else {
    //None
    //};

    //// read the actual dictionary id if the flag was present
    //let dictionary = if dictionary {
    //let num = u32::from_le_bytes(read(reader.by_ref())?);
    //Some(num)
    //} else {
    //None
    //};

    //// checksum of the frame header
    //let header_checksum = read_byte(reader.by_ref())?;

    todo!()
}

#[cfg(test)]
mod tests {
    use crate::{ArrayBuf, Buf};

    #[test]
    fn hello() {
        let raw = "BCJNGGRApwYAAIBoZWxsbwoAAAAA+VtrlA==";
        let raw = base64::decode(raw).unwrap();

        let mut buf = ArrayBuf::<u8, 5>::new();
        super::decompress(&raw, &mut buf).unwrap();
        assert_eq!(core::str::from_utf8(buf.as_slice()), Ok("hello"));
    }
}
