//! Safe and fast Lz4 compression implemented in `no_std` Rust.
#![forbid(unsafe_code)]
#![feature(min_const_generics)]
// #![deny(warnings, missing_docs)]
// #![no_std]

#[cfg(any(feature = "alloc", test))]
extern crate alloc;

mod decompress;
pub use decompress::*;

mod compress;
pub use compress::*;

mod buf;
pub use buf::*;

/// Provides the maximum size that LZ4 compression may output in a "worst case" scenario.
///
/// This function is mostly useful to allocate enough memory.
/// Returns 0 if the input size is 0 and `None` if the input size is too large.
pub const fn compressed_bound(size: usize) -> Option<usize> {
    // 2.113.929.216 bytes
    const MAX_INPUT_SIZE: usize = 0x7E000000;

    if size > MAX_INPUT_SIZE {
        None
    } else if size == 0 {
        Some(0)
    } else {
        Some(size + (size / 255) + 16)
    }
}
