//! Vec-Like Operations that also work in `no_std` and without `alloc`.

use core::ops;

/// Represents anything that can be used to store the resulting data
/// of a LZ4 operation.
///
/// Contains different required Vec-Like operations, that are also
/// applyable to an array or slice.
pub trait Buf<T> {
    /// Push an item to the end of this buffer.
    ///
    /// Returns the given `item`, if there is no capacity
    /// left to push this item.
    fn push(&mut self, item: T) -> Option<T>;

    /// Reserves capacity for at least `count` elements that can be inserted
    /// into this buffer.
    ///
    /// Returns `true` if it was able to reserve enough memory,
    /// and `false` if there's not enough memory left.
    fn reserve(&mut self, count: usize) -> bool;

    /// Return the number of initialized elements in this buffer.
    fn len(&self) -> usize;

    /// Extends this buffer with the items contained in the iterator.
    ///
    /// It also requires the length of the iterator, to pre-reserve
    /// the space that is required for the operation.
    ///
    /// Returns `true` if it was able to reserve enough memory,
    /// and `false` if there's not enough memory left.
    fn extend<I>(&mut self, len: usize, iter: I) -> bool
    where
        I: IntoIterator<Item = T>,
    {
        if !self.reserve(len) {
            false
        } else {
            iter.into_iter().for_each(|item| {
                self.push(item);
            });
            true
        }
    }

    /// Resizes this buffer so that the new length is equal to `len`.
    ///
    /// If this buffers len is greater than the given len, the required elements
    /// are filled by cloning the given item.
    ///
    /// Returns `true` if it was able to reserve enough memory,
    /// and `false` if there's not enough memory left.
    fn resize(&mut self, len: usize, item: T) -> bool
    where
        T: Clone,
    {
        let diff = if len > self.len() {
            len - self.len()
        } else {
            return false;
        };

        if !self.reserve(diff) {
            return false;
        }

        (0..diff).map(|_| item.clone()).for_each(|item| {
            self.push(item);
        });
        true
    }
}

/// A `Buf` implementation that uses a fixed size array as the backing storage.
#[derive(Clone, Copy)]
pub struct ArrayBuf<T, const N: usize> {
    arr: [T; N],
    len: usize,
}

impl<T, const N: usize> ArrayBuf<T, N> {
    /// Create a new `ArrayBuf` using the `Default` and `Copy` implementations
    /// to fill the array.
    pub fn new() -> Self
    where
        T: Default + Copy,
    {
        Self {
            arr: [T::default(); N],
            len: 0,
        }
    }

    /// Returns a slice to the inner storage of this buf.
    pub fn as_slice(&self) -> &[T] {
        &self.arr
    }

    /// Returns a mutable slice to the inner storage of this buf.
    pub fn as_mut_slice(&mut self) -> &mut [T] {
        &mut self.arr
    }
}

impl<T, const N: usize> Buf<T> for ArrayBuf<T, N> {
    fn push(&mut self, item: T) -> Option<T> {
        let entry = match self.arr.get_mut(self.len) {
            Some(entry) => entry,
            None => return Some(item),
        };
        *entry = item;
        self.len += 1;
        None
    }

    fn reserve(&mut self, count: usize) -> bool {
        self.len + count <= N
    }

    fn len(&self) -> usize {
        self.len
    }
}

impl<T, const N: usize> ops::Deref for ArrayBuf<T, N> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl<T, const N: usize> ops::DerefMut for ArrayBuf<T, N> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.as_mut_slice()
    }
}

impl<T, const N: usize> AsRef<[T]> for ArrayBuf<T, N> {
    fn as_ref(&self) -> &[T] {
        self
    }
}

#[cfg(test)]
mod tests {
    use super::{ArrayBuf, Buf};

    #[test]
    fn array_buf() {
        let mut buf = ArrayBuf::<u8, 4>::new();

        assert!(buf.reserve(4));
        assert!(!buf.reserve(5));

        assert!(buf.push(1).is_none());
        assert!(buf.push(2).is_none());
        assert!(buf.push(3).is_none());
        assert!(buf.push(4).is_none());
        assert!(buf.push(5).is_some());

        assert_eq!(buf.len(), 4);

        let mut buf = ArrayBuf::<u8, 4>::new();
        assert!(buf.resize(4, 0));
        assert!(!buf.resize(5, 0));
    }
}
