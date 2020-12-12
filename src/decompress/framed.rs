//! Decompression of the LZ4 [Frame Format]
//!
//! [Frame Format]: https://github.com/lz4/lz4/blob/dev/doc/lz4_Frame_format.md

#![allow(non_upper_case_globals)]

use super::{ByteIter, DecompressError};
use crate::Buf;
use bitflags::bitflags;
use core::hash::Hasher;
use twox_hash::XxHash32;

/// The highest bit indicates if data is compressed or uncompressed.
const UNCOMPRESSED_DATA: u32 = 1 << 31;

bitflags! {
    struct Flags: u8 {
        const IndependentBlocks = 0b00100000;
        const BlockChecksums    = 0b00010000;
        const ContentSize       = 0b00001000;
        const ContentChecksum   = 0b00000100;
        const DictionaryId      = 0b00000001;
    }
}

fn parse_flags(raw: u8) -> Result<Flags, DecompressError> {
    // first two bits represent the version that was used
    // to compress the data
    let version = raw >> 6;

    if version != super::VERSION {
        return Err(DecompressError::VersionNotSupported);
    }

    // bit 1 is reserved and should always be 0
    if (raw & 0b10) != 0 {
        return Err(DecompressError::ReservedBitHigh);
    }

    Ok(Flags::from_bits_truncate(raw))
}

/// This method can be used to decompress data that is compressed using
/// the LZ4 [Frame Format].
///
/// If you want a streaming decompresser, you have to enable `std` feature
/// and use [`stream::Decompresser`](crate::decompress::stream::Decompressor).
///
/// [Frame Format]: https://github.com/lz4/lz4/blob/dev/doc/lz4_Frame_format.md
pub fn decompress<B: Buf<u8>>(input: &[u8], out: &mut B) -> Result<(), DecompressError> {
    let mut reader = ByteIter::new(input);

    let magic = u32::from_le_bytes(reader.read()?);
    if magic != super::MAGIC {
        return Err(DecompressError::InvalidMagic);
    }

    let mut hasher = XxHash32::with_seed(0);

    let flags = reader.read_byte()?;
    hasher.write_u8(flags);
    let flags = parse_flags(flags)?;

    let block_descriptor = reader.read_byte()?;
    hasher.write_u8(block_descriptor);
    // check if all reserved bits are zero
    if (block_descriptor & 0b10001111) != 0 {
        return Err(DecompressError::ReservedBitHigh);
    }

    let max_block_size = ((block_descriptor >> 4) & 0b111) as usize;
    let max_block_size = match max_block_size {
        4..=7 => 1 << (max_block_size * 2 + 8),
        _ => return Err(DecompressError::InvalidMaxBlockSize),
    };

    let content_size = if flags.contains(Flags::ContentSize) {
        let size = u64::from_le_bytes(reader.read()?);
        hasher.write_u64(size);
        Some(size)
    } else {
        None
    };

    assert!(
        !flags.contains(Flags::DictionaryId),
        "Dictionary IDs are currently not supported"
    );

    let header_checksum = reader.read_byte()?;
    let actual_hash = (hasher.finish() >> 8) as u8;
    if header_checksum != actual_hash {
        return Err(DecompressError::HeaderChecksumInvalid);
    }

    loop {
        let size = u32::from_le_bytes(reader.read()?);

        // `0` is the end marker and indicates the end of the stream of blocks.
        if size == 0 {
            break;
        }

        let is_uncompressed = size & UNCOMPRESSED_DATA != 0;
        let size = size & !UNCOMPRESSED_DATA;

        let mut hash = None;
        let mut hash_slice = |slice: &[u8]| {
            if !flags.contains(Flags::BlockChecksums) {
                return;
            }

            let mut hasher = XxHash32::with_seed(0);
            hasher.write(slice);
            hash = Some(hasher.finish() as u32);
        };

        match size {
            // if the highest bit is set, this is uncompressed data
            size if is_uncompressed => {
                let source = reader.take(size as usize)?;
                hash_slice(source);

                if !out.extend(source) {
                    return Err(DecompressError::MemoryLimitExceeded);
                }
            }
            // if block is larger by max block size, treat it as uncompressed data
            size if size > max_block_size => {
                let source = reader.take(size as usize)?;
                hash_slice(source);
                if !out.extend(source) {
                    return Err(DecompressError::MemoryLimitExceeded);
                }
            }
            // compressed data
            size => {
                let block = reader.take(size as usize)?;
                hash_slice(block);
                super::raw::decompress_block(block, out)?;
            }
        };

        if let Some(actual) = hash.take() {
            assert!(flags.contains(Flags::BlockChecksums));
            let expected = u32::from_le_bytes(reader.read()?);
            if actual != expected {
                return Err(DecompressError::BlockChecksumInvalid);
            }
        }
    }

    if flags.contains(Flags::ContentChecksum) {
        let mut hasher = XxHash32::with_seed(0);
        hasher.write(out.as_slice());
        let expected = hasher.finish() as u32;

        let actual = u32::from_le_bytes(reader.read()?);
        if actual != expected {
            return Err(DecompressError::ContentChecksumInvalid);
        }
    }

    if let Some(expected) = content_size {
        if expected as usize != out.len() {
            return Err(DecompressError::ContentSizeInvalid);
        }
    }

    Ok(())
}
