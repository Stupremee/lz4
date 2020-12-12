//! Decompression of the LZ4 [Frame Format]
//!
//! [Frame Format]: https://github.com/lz4/lz4/blob/dev/doc/lz4_Frame_format.md

#![allow(non_upper_case_globals)]

use super::{ByteIter, Error};
use crate::Buf;
use bitflags::bitflags;
use core::hash::Hasher;
use twox_hash::XxHash32;

bitflags! {
    struct Flags: u8 {
        const IndependentBlocks = 0b00100000;
        const BlockChecksums    = 0b00010000;
        const ContentSize       = 0b00001000;
        const ContentChecksum   = 0b00000100;
        const DictionaryId      = 0b00000001;
    }
}

fn parse_flags(raw: u8) -> Result<Flags, Error> {
    // first two bits represent the version that was used
    // to compress the data
    let version = raw >> 6;

    if version != super::VERSION {
        return Err(Error::VersionNotSupported);
    }

    // bit 1 is reserved and should always be 0
    if (raw & 0b10) != 0 {
        return Err(Error::ReservedBitHigh);
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
pub fn decompress<B: Buf<u8>>(input: &[u8], out: &mut B) -> Result<(), Error> {
    let mut reader = ByteIter::new(input);

    let magic = u32::from_le_bytes(reader.read()?);
    if magic != super::MAGIC {
        return Err(Error::InvalidMagic);
    }

    let mut hasher = XxHash32::with_seed(0);

    let flags = reader.read_byte()?;
    hasher.write_u8(flags);
    let flags = parse_flags(flags)?;

    let block_descriptor = reader.read_byte()?;
    hasher.write_u8(block_descriptor);
    // check if all reserved bits are zero
    if (block_descriptor & 0b10001111) != 0 {
        return Err(Error::ReservedBitHigh);
    }

    let max_block_size = ((block_descriptor >> 4) & 0b111) as usize;
    let max_block_size = match max_block_size {
        4..=7 => 1 << (max_block_size * 2 + 8),
        _ => return Err(Error::InvalidMaxBlockSize),
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
        return Err(Error::HeaderChecksumInvalid);
    }

    loop {
        let size = u32::from_le_bytes(reader.read()?);

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
            // `0` is the end marker and indicates the end of
            // the stream of blocks
            0 => {
                if flags.contains(Flags::BlockChecksums) {
                    // TODO: I guess this can be replaced with a
                    let hasher = XxHash32::with_seed(0);
                    let actual = hasher.finish() as u32;
                    let expected = u32::from_le_bytes(reader.read()?);
                    if actual != expected {
                        return Err(Error::BlockChecksumInvalid);
                    }
                }

                break;
            }
            // if the highest bit is set, this is uncompressed data
            size if size & 0x80000000 != 0 => {
                let real_size = size & 0x7FFFFFFF;
                let source = reader.take(real_size as usize)?;
                hash_slice(source);

                if !out.extend(source) {
                    return Err(Error::MemoryLimitExceeded);
                }
            }
            // if block is larger by max block size, treat it as uncompressed data
            size if size > max_block_size => {
                let source = reader.take(size as usize)?;
                hash_slice(source);
                if !out.extend(source) {
                    return Err(Error::MemoryLimitExceeded);
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
                return Err(Error::BlockChecksumInvalid);
            }
        }
    }

    if flags.contains(Flags::ContentChecksum) {
        let mut hasher = XxHash32::with_seed(0);
        hasher.write(out.as_slice());
        let expected = hasher.finish() as u32;

        let actual = u32::from_le_bytes(reader.read()?);
        if actual != expected {
            return Err(Error::ContentChecksumInvalid);
        }
    }

    Ok(())
}
