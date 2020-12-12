//! Implementation of decompression of lz4 compressed data.

use core::fmt;

mod iter;
pub(crate) use iter::ByteIter;

mod framed;
pub use framed::*;

mod raw;
pub use raw::*;

/// The magic number which is at the start of every
/// compressed data in the frame format.
const MAGIC: u32 = 0x184D2204;

/// The version this decompresser is capable of decompressing.
const VERSION: u8 = 0b01;

/// The error type that is returned by various decompression-related methods.
#[derive(Clone, Copy, Debug)]
#[non_exhaustive]
pub enum DecompressError {
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
    /// A reserved bit was 1.
    ///
    /// This is either caused by invalid input, or by trying to
    /// decompress data that was compressed using a newer/older
    /// version of the spec.
    ReservedBitHigh,
    /// Tried to decompress frame header which contained an illegal
    /// number for the maximum block size.
    InvalidMaxBlockSize,
    /// The checksum check for the frame header failed.
    HeaderChecksumInvalid,
    /// The checksum check for a block failed.
    BlockChecksumInvalid,
    /// The checksum check for the decompressed content failed.
    ContentChecksumInvalid,
    /// The content size that was provided in the frame header doesn't
    /// match the actual output size.
    ContentSizeInvalid,
}

impl fmt::Display for DecompressError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DecompressError::MemoryLimitExceeded => f.write_str("not enough memory available in out pointer"),
            DecompressError::UnexpectedEof => {
                f.write_str("expected at lelast one byte to be read, but instead end was reached")
            }
            DecompressError::ZeroMatchOffset => f.write_str(
                "The offset was zero. This is most likely caused by trying to parse invalid input.",
            ),

            DecompressError::InvalidMagic => f.write_str(
                "The magic number is invalid. This is most likely caused by trying to parse invalid input.",
            ),
            DecompressError::VersionNotSupported => f.write_str("The data was comrpessed using a version of LZ4 that is not supported."),
            DecompressError::InvalidInput => f.write_str("The provided data is invalid."),
            DecompressError::ReservedBitHigh => f.write_str("One of the reserved bits was 1."),
            DecompressError::InvalidMaxBlockSize => f.write_str("Maximum block size is invalid"),
            DecompressError::HeaderChecksumInvalid => f.write_str("Frame header checksum verification failed."),
            DecompressError::BlockChecksumInvalid => f.write_str("Block checksum verification failed."),
            DecompressError::ContentChecksumInvalid => f.write_str("Content checksum verification failed."),
            DecompressError::ContentSizeInvalid => f.write_str("Content size verification failed."),
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::{ArrayBuf, Buf};

    #[test]
    fn hello() {
        let raw = "BCJNGGRApwYAAIBoZWxsbwoAAAAA+VtrlA==";
        let raw = base64::decode(raw).unwrap();

        let mut buf = ArrayBuf::<u8, 6>::new();
        super::decompress(&raw, &mut buf).unwrap();
        assert_eq!(core::str::from_utf8(buf.as_slice()), Ok("hello\n"));
    }
}
