//! Vec-Like Operations that also work in `no_std` and without `alloc`.

/// Represents anything that can be used to store the resulting data
/// of a LZ4 operation.
///
/// Contains different required Vec-Like operations, that are also
/// applyable to an array or slice.
pub trait Buf<T: Copy> {
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

    /// Returns a slice to the inner storage of this buf.
    fn as_slice(&self) -> &[T];

    /// Returns a mutable slice to the inner storage of this buf.
    fn as_mut_slice(&mut self) -> &mut [T];

    /// Extends this buffer with the items contained in the slice.
    ///
    /// Returns `true` if it was able to reserve enough memory,
    /// and `false` if there's not enough memory left.
    fn extend(&mut self, buf: &[T]) -> bool;

    /// Resizes this buffer so that the new length is equal to `len`.
    ///
    /// If this buffers len is greater than the given len, the required elements
    /// are filled by cloning the given item. Note that this will not truncate
    /// the buf if `len` is smaller then this bufs len.
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
#[derive(Clone)]
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
}

impl<T: Copy, const N: usize> Buf<T> for ArrayBuf<T, N> {
    fn push(&mut self, item: T) -> Option<T> {
        let entry = match self.arr.get_mut(self.len) {
            Some(entry) => entry,
            None => return Some(item),
        };
        *entry = item;
        self.len += 1;
        None
    }

    fn extend(&mut self, buf: &[T]) -> bool {
        if !self.reserve(buf.len()) {
            false
        } else {
            let slice = &mut self.arr[self.len..self.len + buf.len()];
            slice.copy_from_slice(buf);
            self.len += buf.len();
            true
        }
    }

    fn reserve(&mut self, count: usize) -> bool {
        self.len + count <= N
    }

    fn len(&self) -> usize {
        self.len
    }

    fn as_slice(&self) -> &[T] {
        &self.arr[..self.len]
    }

    fn as_mut_slice(&mut self) -> &mut [T] {
        &mut self.arr[..self.len]
    }
}

#[cfg(any(feature = "alloc", test))]
pub use heap::*;

#[cfg(any(feature = "alloc", test))]
mod heap {
    use super::Buf;
    use alloc::vec::Vec;

    /// A `Buf` that will dynamically allocate the memory on the heap.
    ///
    /// This struct is only available with the `alloc` feature enabled.
    #[derive(Clone)]
    pub struct HeapBuf<T>(Vec<T>);

    impl<T> HeapBuf<T> {
        /// Create a new `HeapBuf`.
        pub fn new() -> Self {
            Self(Vec::new())
        }

        /// Create a new `HeapBuf` with the specified capacity.
        pub fn with_capacity(cap: usize) -> Self {
            Self(Vec::with_capacity(cap))
        }
    }

    impl<T: Copy> Buf<T> for HeapBuf<T> {
        fn push(&mut self, item: T) -> Option<T> {
            self.0.push(item);
            None
        }

        fn extend(&mut self, buf: &[T]) -> bool {
            self.reserve(buf.len());
            self.0.extend_from_slice(buf);
            true
        }

        fn reserve(&mut self, count: usize) -> bool {
            self.0.reserve(count);
            true
        }

        fn len(&self) -> usize {
            self.0.len()
        }

        fn as_slice(&self) -> &[T] {
            &self.0
        }

        fn as_mut_slice(&mut self) -> &mut [T] {
            &mut self.0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{ArrayBuf, Buf, HeapBuf};

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
        assert!(!buf.resize(4, 0));
    }

    #[test]
    fn heap_buf() {
        let mut buf = HeapBuf::<u8>::new();

        assert!(buf.reserve(4));
        assert!(buf.reserve(5));

        assert!(buf.push(1).is_none());
        assert!(buf.push(2).is_none());
        assert!(buf.push(3).is_none());
        assert!(buf.push(4).is_none());
        assert!(buf.push(5).is_none());

        assert_eq!(buf.len(), 5);

        let mut buf = HeapBuf::<u8>::new();
        assert!(buf.resize(6, 0));
        assert!(buf.resize(7, 0));
    }
}
