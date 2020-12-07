//! Safe and fast Lz4 compression implemented in `no_std` Rust.
// #![deny(unsafe_code, warnings, missing_docs)]
#![no_std]

#[cfg(feature = "alloc")]
extern crate alloc;

use uninit::out_ref::Out;

/// ...
pub fn decompress<'out>(data: &'_ [u8], out: Out<'out, [u8]>) -> &'out [u8] {
    unimplemented!()
}
