//! Simple Iterator over slice of arrays that makes
//! decompressing much easier.

use super::DecompressError;

pub(crate) struct ByteIter<'input> {
    bytes: &'input [u8],
    idx: usize,
}

impl<'input> ByteIter<'input> {
    pub(crate) fn new(bytes: &'input [u8]) -> Self {
        Self { bytes, idx: 0 }
    }

    pub(crate) fn take(&mut self, count: usize) -> Result<&[u8], DecompressError> {
        let bytes = self
            .bytes
            .get(self.idx..self.idx + count)
            .ok_or(DecompressError::UnexpectedEof)?;
        self.idx += count;
        Ok(bytes)
    }

    pub(crate) fn read_byte(&mut self) -> Result<u8, DecompressError> {
        let byte = self
            .bytes
            .get(self.idx)
            .ok_or(DecompressError::UnexpectedEof)?;
        self.idx += 1;
        Ok(*byte)
    }

    pub(crate) fn read<const N: usize>(&mut self) -> Result<[u8; N], DecompressError> {
        let mut buf = [0u8; N];
        for entry in buf.iter_mut() {
            *entry = self.read_byte()?;
        }
        Ok(buf)
    }

    pub(crate) fn read_int(&mut self, first: usize) -> Result<usize, DecompressError> {
        if first != 15 {
            return Ok(first);
        }

        let mut x = 15usize;
        loop {
            let byte = self.read_byte()?;
            x += byte as usize;
            if byte != 255 {
                break;
            }
        }
        Ok(x)
    }
}

#[cfg(test)]
mod tests {
    use super::ByteIter;

    #[test]
    fn take() {
        let mut iter = ByteIter::new(&[1, 2, 3, 4, 5, 6]);
        assert_eq!(iter.read_byte().unwrap(), 1);
        assert_eq!(iter.take(3).unwrap(), &[2, 3, 4]);
        assert_eq!(u16::from_le_bytes(iter.read().unwrap()), (6 << 8) | 5);
    }
}
