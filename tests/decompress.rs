use lz4::{Buf, HeapBuf};
use std::io;
use twox_hash::xxh3::hash64;

macro_rules! data_path {
    ($name:literal) => {
        concat!(env!("CARGO_MANIFEST_DIR"), "/tests/data/", $name)
    };
}

fn test_file_pair(compressed_path: &str, decompressed_path: &str) -> io::Result<()> {
    let compressed = std::fs::read(compressed_path)?;
    let decompressed = std::fs::read(decompressed_path)?;

    let mut buf = HeapBuf::new();
    lz4::decompress(&compressed, &mut buf).expect("failed to decompress file");

    assert_eq!(
        hash64(&decompressed),
        hash64(buf.as_slice()),
        "{} doesn't match it's decompressed data",
        decompressed_path
    );

    Ok(())
}

#[test]
fn test_wallpaper() {
    test_file_pair(
        data_path!("wallpaper_compressed.jpg"),
        data_path!("wallpaper.jpg"),
    )
    .expect("I/O error");
}

#[test]
fn test_64mb_zero() {
    test_file_pair(
        data_path!("64_MB_zero_compressed.bin"),
        data_path!("64_MB_zero.bin"),
    )
    .expect("I/O error");
}

#[test]
fn test_64mb_random() {
    test_file_pair(data_path!("64_MB_compressed.bin"), data_path!("64_MB.bin")).expect("I/O error");
}
