//! A self-expanding ring buffer optimized for working with slices of data. This functions
//! similarly to [`VecDeque`], but with handy methods for efficiently working with slices of
//! data. This can be especially useful when working with streams of data where the input and
//! output buffers are different sizes.
//!
//! Copies/reads with slices are implemented with memcpy. This algorithm attempts to use as
//! little memcpys and allocations as possible, and only potentially shuffles data around when
//! the capacity of the buffer is increased.
//!
//! This buffer does not contain any Producer/Consumer logic, but it could be used as a building
//! block for a ring buffer that does.
//!
//! This crate can also be used without the standard library (`#![no_std]`).
//!
//! ## Example
//! ```rust
//! use core::num::NonZeroUsize;
//! use expanding_slice_rb::ExpSliceRB;
//!
//! // Create a ring buffer with type u32. There is no data in the buffer to start.
//! //
//! // If possible, it is a good idea to set `capacity` to the largest you expect the
//! // buffer to get to avoid future memory allocations.
//! let mut buf = ExpSliceRB::<u32>::with_capacity(NonZeroUsize::new(3).unwrap());
//!
//! let data = [0u32, 1, 2];
//!
//! // Memcpy data from a slice into the ring buffer. The buffer will automatically
//! // expand to fill new data.
//! buf.write(&data);
//! assert_eq!(buf.len(), 3);
//! assert_eq!(buf.capacity().get(), 3);
//!
//! buf.write(&data);
//! assert_eq!(buf.len(), 6);
//! assert_eq!(buf.capacity().get(), 6);
//!
//! // Memcpy the next chunk of data into the read slice. If the length of existing
//! // data in the buffer is less than the length of the slice, then only that amount
//! // of data will be copied into the front of the slice.
//! //
//! // This is streaming, meaning the copied data will be cleared from the buffer for reuse, and
//! // the next call to `read_into()` will start copying from where the previous call left off.
//! // If you don't want this behavior, use the `peek_into()` method.
//! let mut read_slice = [5u32; 4];
//! let mut amount_written = buf.read_into(&mut read_slice);
//! assert_eq!(amount_written, 4);
//! assert_eq!(read_slice, [0u32, 1, 2, 0]);
//!
//! buf.write(&data);
//! let mut large_read_slice = [5u32; 8];
//! amount_written = buf.read_into(&mut large_read_slice);
//! assert_eq!(amount_written, 5);
//! assert_eq!(large_read_slice, [1u32, 2, 0, 1, 2, 5, 5, 5]);
//! ```
//!
//! [`VecDeque`]: https://doc.rust-lang.org/std/collections/struct.VecDeque.html

#![no_std]

extern crate alloc;

use core::num::NonZeroUsize;

use alloc::vec::Vec;
use slice_ring_buf::SliceRB;

/// A self-expanding ring buffer optimized for working with slices of data. This functions
/// similarly to [`VecDeque`], but with handy methods for efficiently working with slices of
/// data. This can be especially useful when working with streams of data where the input and
/// output buffers are different sizes.
///
/// Copies/reads with slices are implemented with memcpy. This algorithm attempts to use as
/// little memcpys and allocations as possible, and only potentially shuffles data around when
/// the capacity of the buffer is increased.
///
/// This buffer does not contain any Producer/Consumer logic, but it could be used as a building
/// block for a ring buffer that does.
///
/// This crate can also be used without the standard library (`#![no_std]`).
///
/// ## Example
/// ```rust
/// # use core::num::NonZeroUsize;
/// # use expanding_slice_rb::ExpSliceRB;
/// // Create a ring buffer with type u32. There is no data in the buffer to start.
/// //
/// // If possible, it is a good idea to set `capacity` to the largest you expect the
/// // buffer to get to avoid future memory allocations.
/// let mut buf = ExpSliceRB::<u32>::with_capacity(NonZeroUsize::new(3).unwrap());
///
/// let data = [0u32, 1, 2];
///
/// // Memcpy data from a slice into the ring buffer. The buffer will automatically
/// // expand to fill new data.
/// buf.write(&data);
/// assert_eq!(buf.len(), 3);
/// assert_eq!(buf.capacity().get(), 3);
///
/// buf.write(&data);
/// assert_eq!(buf.len(), 6);
/// assert_eq!(buf.capacity().get(), 6);
///
/// // Memcpy the next chunk of data into the read slice. If the length of existing
/// // data in the buffer is less than the length of the slice, then only that amount
/// // of data will be copied into the front of the slice.
/// //
/// // This is streaming, meaning the copied data will be cleared from the buffer for reuse, and
/// // the next call to `read_into()` will start copying from where the previous call left off.
/// // If you don't want this behavior, use the `peek_into()` method instead.
/// let mut read_slice = [5u32; 4];
/// let mut amount_written = buf.read_into(&mut read_slice);
/// assert_eq!(amount_written, 4);
/// assert_eq!(read_slice, [0u32, 1, 2, 0]);
///
/// buf.write(&data);
/// let mut large_read_slice = [5u32; 8];
/// amount_written = buf.read_into(&mut large_read_slice);
/// assert_eq!(amount_written, 5);
/// assert_eq!(large_read_slice, [1u32, 2, 0, 1, 2, 5, 5, 5]);
/// ```
///
/// [`VecDeque`]: https://doc.rust-lang.org/std/collections/struct.VecDeque.html
pub struct ExpSliceRB<T: Default + Clone + Copy> {
    buffer: SliceRB<T>,
    index: isize,
    data_len: usize,
}

impl<T: Default + Clone + Copy> ExpSliceRB<T> {
    /// Create a new empty [`ExpSliceRB`] from the given vec.
    ///
    /// # Panics
    ///
    /// Panics if vec.len() is equal to 0 or is greater than isize::MAX.
    pub fn from_vec(vec: Vec<T>) -> Self {
        Self {
            buffer: SliceRB::from_vec(vec),
            index: 0,
            data_len: 0,
        }
    }

    /// Create a new empty [`ExpSliceRB`] with an initial allocated capacity.
    ///
    /// If possible, it is a good idea to set `capacity` to the largest you expect the
    /// buffer to get to avoid future memory allocations.
    ///
    /// This allocates new memory and is ***not*** real-time safe.
    ///
    /// # Example
    /// ```rust
    /// # use core::num::NonZeroUsize;
    /// # use expanding_slice_rb::ExpSliceRB;
    /// let buf = ExpSliceRB::<u32>::with_capacity(NonZeroUsize::new(128).unwrap());
    ///
    /// assert_eq!(buf.len(), 0);
    /// assert_eq!(buf.capacity().get(), 128);
    /// ```
    ///
    /// # Panics
    ///
    /// * This will panic if `capacity > isize::MAX`.
    /// * This will panic if allocation fails due to being out of memory.
    /// [`ExpSliceRB`]: struct.ExpSliceRB.html
    pub fn with_capacity(capacity: NonZeroUsize) -> Self {
        // Safe because our algorithm ensures data will always be written to
        // before being read.
        let buffer = unsafe { SliceRB::new_uninit(capacity) };

        Self {
            buffer,
            index: 0,
            data_len: 0,
        }
    }

    /// Reads the next chunk of existing data into the given slice. If the length of existing
    /// data in the buffer is less than the length of the slice, then only that amount of data
    /// will be copied into the front of the slice.
    ///
    /// This is streaming, meaning the copied data will be cleared from the buffer for reuse, and
    /// the next call to `read_into()` will start copying from where the previous call left off.
    /// If you don't want this behavior, use the `peek_into()` method instead.
    ///
    /// This does not allocate any memory and is real-time safe.
    ///
    /// ## Returns
    /// This returns the total amount of data that was copied into `slice`.
    ///
    /// # Example
    /// ```rust
    /// # use core::num::NonZeroUsize;
    /// # use expanding_slice_rb::ExpSliceRB;
    /// let mut buf = ExpSliceRB::<u32>::with_capacity(NonZeroUsize::new(6).unwrap());
    ///
    /// let data = [0u32, 1, 2];
    /// buf.write(&data);
    /// buf.write(&data);
    ///
    /// let mut read_slice = [5u32; 4];
    /// let mut amount_written = buf.read_into(&mut read_slice);
    /// assert_eq!(amount_written, 4);
    /// assert_eq!(read_slice, [0u32, 1, 2, 0]);
    ///
    /// buf.write(&data);
    /// let mut large_read_slice = [5u32; 8];
    /// amount_written = buf.read_into(&mut large_read_slice);
    /// assert_eq!(amount_written, 5);
    /// assert_eq!(large_read_slice, [1u32, 2, 0, 1, 2, 5, 5, 5]);
    /// ```
    pub fn read_into(&mut self, mut slice: &mut [T]) -> usize {
        // No data in buffer.
        if self.data_len == 0 {
            return 0;
        }

        if self.data_len <= slice.len() {
            // Copy all remaining data into the slice.

            let amount_to_copy = self.data_len;
            slice = &mut slice[0..amount_to_copy];

            // Copy the data.
            self.buffer.read_into(slice, self.index);

            // Advance the index.
            self.index = self.buffer.constrain(self.index + amount_to_copy as isize);
            self.data_len = 0;

            return amount_to_copy;
        }

        // Else copy up to the length of the slice.
        self.buffer.read_into(slice, self.index);

        // Advance the index.
        self.index = self.buffer.constrain(self.index + slice.len() as isize);
        self.data_len -= slice.len();

        slice.len()
    }

    /// Reads the next chunk of existing data into the given slice. If the length of existing
    /// data in the buffer is less than the length of the slice, then only that amount of data
    /// will be copied into the front of the slice.
    ///
    /// As apposed to `read_into()`, this method is ***not*** streaming and does not effect the
    /// length of existing data in the buffer.
    ///
    /// This does not allocate any memory and is real-time safe.
    ///
    /// ## Returns
    /// This returns the total amount of data that was copied into `slice`.
    ///
    /// # Example
    /// ```rust
    /// # use core::num::NonZeroUsize;
    /// # use expanding_slice_rb::ExpSliceRB;
    /// let mut buf = ExpSliceRB::<u32>::with_capacity(NonZeroUsize::new(6).unwrap());
    ///
    /// let data = [0u32, 1, 2];
    /// buf.write(&data);
    /// buf.write(&data);
    /// assert_eq!(buf.len(), 6);
    ///
    /// let mut read_slice = [5u32; 4];
    /// let mut amount_written = buf.peek_into(&mut read_slice);
    /// assert_eq!(amount_written, 4);
    /// assert_eq!(read_slice, [0u32, 1, 2, 0]);
    /// assert_eq!(buf.len(), 6);
    /// ```
    ///
    /// ## Returns
    /// This returns the total amount of data that was copied into `slice`.
    pub fn peek_into(&mut self, mut slice: &mut [T]) -> usize {
        // No data in buffer.
        if self.data_len == 0 {
            return 0;
        }

        if self.data_len <= slice.len() {
            // Copy all remaining data into the slice.

            let amount_to_copy = self.data_len;
            slice = &mut slice[0..amount_to_copy];

            // Copy the data.
            self.buffer.read_into(slice, self.index);

            return amount_to_copy;
        }

        // Else copy up to the length of the slice.
        self.buffer.read_into(slice, self.index);

        slice.len()
    }

    /// Append additional data into the buffer to be read later. More memory may be allocated
    /// if the buffer is not large enough.
    ///
    /// This may allocate new memory and is ***not*** real-time safe.
    ///
    /// # Example
    /// ```rust
    /// # use core::num::NonZeroUsize;
    /// # use expanding_slice_rb::ExpSliceRB;
    /// let mut buf = ExpSliceRB::<u32>::with_capacity(NonZeroUsize::new(6).unwrap());
    ///
    /// let data = [0u32, 1, 2];
    ///
    /// buf.write(&data);
    /// assert_eq!(buf.len(), 3);
    /// assert_eq!(buf.capacity().get(), 6);
    ///
    /// buf.write(&data);
    /// assert_eq!(buf.len(), 6);
    /// assert_eq!(buf.capacity().get(), 6);
    ///
    /// buf.write(&data);
    /// assert_eq!(buf.len(), 9);
    /// assert_eq!(buf.capacity().get(), 9);
    /// ```
    ///
    /// # Panics
    ///
    /// * This will panic if `capacity > isize::MAX`.
    /// * This will panic if allocation fails due to being out of memory.
    pub fn write(&mut self, slice: &[T]) {
        let new_len = self.data_len + slice.len();

        // Expand the buffer if the new length is greater than the buffer length.
        if new_len > self.buffer.len().get() {
            self.reserve(new_len - self.buffer.len().get());
        }

        // Write the data into the buffer.
        self.buffer
            .write_latest(slice, self.index + self.data_len as isize);

        self.data_len = new_len;
    }

    /// Append additional data into the buffer to be read later. If the data cannot fit
    /// into the buffer, then no data is copied and and error is returned.
    ///
    /// This does not allocate any memory and is real-time safe.
    ///
    /// # Example
    /// ```rust
    /// # use core::num::NonZeroUsize;
    /// # use expanding_slice_rb::ExpSliceRB;
    /// let mut buf = ExpSliceRB::<u32>::with_capacity(NonZeroUsize::new(6).unwrap());
    ///
    /// let data = [0u32, 1, 2];
    ///
    /// assert_eq!(buf.try_write(&data), Ok(()));
    /// assert_eq!(buf.try_write(&data), Ok(()));
    /// assert_eq!(buf.try_write(&data), Err(()));
    /// ```
    pub fn try_write(&mut self, slice: &[T]) -> Result<(), ()> {
        let new_len = self.data_len + slice.len();

        if new_len > self.buffer.len().get() {
            return Err(());
        }

        // Write the data into the buffer.
        self.buffer
            .write_latest(slice, self.index + self.data_len as isize);

        self.data_len = new_len;

        Ok(())
    }

    /// Reserves capacity for at least `additional` more elements to be inserted
    /// into the buffer.
    ///
    /// Due to the algorithm, no data will actually be initialized. However, more memory
    /// may need to be allocated.
    ///
    /// This may allocate new memory and is ***not*** real-time safe.
    ///
    /// # Panics
    ///
    /// * This will panic if `capacity > isize::MAX`.
    /// * This will panic if allocation fails due to being out of memory.
    pub fn reserve(&mut self, additional: usize) {
        if additional == 0 {
            return;
        }

        let data_end = self.index as usize + self.data_len;
        let prev_buffer_len = self.buffer.len().get();

        // Safe because algorithm ensures data will always be written to
        // before being read.
        unsafe {
            self.buffer
                .set_len_uninit(NonZeroUsize::new(prev_buffer_len + additional).unwrap());
        }

        if data_end > prev_buffer_len {
            // If the existing data wraps around, then copy the wrapped portion to make the
            // existing data contiguous.

            let wrapped_data_len = data_end - prev_buffer_len;
            let start_data_len = self.data_len - wrapped_data_len;

            let (src, dst) = self.buffer.raw_data_mut().split_at_mut(self.index as usize);

            if self.data_len > dst.len() {
                let second_cpy_len = self.data_len - dst.len();

                dst[start_data_len..start_data_len + additional]
                    .copy_from_slice(&src[0..additional]);

                src.copy_within(additional..additional + second_cpy_len, 0);
            } else {
                dst[start_data_len..start_data_len + wrapped_data_len]
                    .copy_from_slice(&src[0..wrapped_data_len]);
            }
        }
    }

    /// Removes all existing data in the buffer.
    ///
    /// This does not allocate any memory and is real-time safe.
    ///
    /// Due to the algorithm, no data will actually be initialized.
    pub fn clear(&mut self) {
        self.index = 0;
        self.data_len = 0;
    }

    /// Removes all existing data in the buffer and sets the allocated capacity of the buffer. This will also call
    /// `Vec::shrink_to_fit()` on the internal Vec.
    ///
    /// Due to the algorithm, no data will actually be initialized.
    ///
    /// This may allocate or deallocate memory and is ***not*** real-time safe.
    ///
    /// # Panics
    ///
    /// * This will panic if `capacity > isize::MAX`.
    /// * This will panic if allocation fails due to being out of memory.
    pub fn clear_and_shrink_to_capacity(&mut self, capacity: NonZeroUsize) {
        self.clear();

        // Safe because algorithm ensures data will always be written to
        // before being read.
        unsafe {
            self.buffer.set_len_uninit(capacity);
        }

        self.buffer.shrink_to_fit();
    }

    /// Returns the allocated capacity of the internal buffer. (This may be different from the allocated
    /// capacity of the internal Vec.)
    pub fn capacity(&self) -> NonZeroUsize {
        self.buffer.len()
    }

    /// Returns the raw allocated capacity of the internal Vec. Note this may be different from the allocated capacity
    /// of the buffer.
    pub fn raw_capacity(&self) -> NonZeroUsize {
        self.buffer.capacity()
    }

    /// Returns the length of existing data in the buffer. This is ***not*** the same as the allocated capacity of the buffer.
    pub fn len(&self) -> usize {
        self.data_len
    }

    /// Returns the amount of unused data available in the buffer.
    ///
    /// This can be useful in conjunction with the `try_write()` method.
    pub fn data_left(&self) -> usize {
        self.buffer.len().get() - self.data_len
    }

    /// Return `true` if the buffer has no existing data, `false` otherwise.
    pub fn is_empty(&self) -> bool {
        self.data_len == 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test() {
        let mut buf: ExpSliceRB<u32> = ExpSliceRB::with_capacity(NonZeroUsize::new(4).unwrap());
        assert_eq!(buf.len(), 0);
        assert_eq!(buf.capacity().get(), 4);

        let data = [0u32, 1, 2, 3];

        buf.write(&data);
        assert_eq!(buf.len(), 4);
        assert_eq!(buf.buffer.raw_data(), data);

        let mut read = [0u32; 4];

        assert_eq!(buf.peek_into(&mut read), 4);
        assert_eq!(read, data);
        assert_eq!(buf.len(), 4);
        assert_eq!(buf.capacity().get(), 4);

        assert_eq!(buf.read_into(&mut read), 4);
        assert_eq!(read, data);
        assert_eq!(buf.len(), 0);
        assert_eq!(buf.capacity().get(), 4);

        let mut read_1 = [5u32; 2];
        let mut read_2 = [5u32; 1];
        let mut read_3 = [5u32; 2];

        buf.write(&data);

        assert_eq!(buf.read_into(&mut read_1), 2);
        assert_eq!(read_1, [0u32, 1]);
        assert_eq!(buf.read_into(&mut read_2), 1);
        assert_eq!(read_2, [2u32]);
        assert_eq!(buf.read_into(&mut read_3), 1);
        assert_eq!(read_3, [3u32, 5]);
        assert_eq!(buf.index, 0);
        assert_eq!(buf.len(), 0);

        buf.write(&data);
        buf.read_into(&mut read_1);
        buf.write(&read_3);
        assert_eq!(buf.len(), 4);
        assert_eq!(buf.capacity().get(), 4);
        assert_eq!(buf.buffer.raw_data(), [3, 5, 2, 3]);
        buf.read_into(&mut read);
        assert_eq!(read, [2, 3, 3, 5]);
        assert_eq!(buf.index, 2);
        assert_eq!(buf.len(), 0);
        assert_eq!(buf.capacity().get(), 4);

        buf.write(&data);
        assert_eq!(buf.buffer.raw_data(), [2, 3, 0, 1]);
        buf.write(&read_3);
        assert_eq!(buf.len(), 6);
        assert_eq!(buf.capacity().get(), 6);
        assert_eq!(buf.buffer.raw_data(), [3, 5, 0, 1, 2, 3]);
        buf.write(&read_2);
        assert_eq!(buf.len(), 7);
        assert_eq!(buf.capacity().get(), 7);
        assert_eq!(buf.buffer.raw_data(), [5, 2, 0, 1, 2, 3, 3]);
        buf.write(&data);
        assert_eq!(buf.len(), 11);
        assert_eq!(buf.capacity().get(), 11);
        assert_eq!(buf.buffer.raw_data(), [2, 3, 0, 1, 2, 3, 3, 5, 2, 0, 1]);

        buf.read_into(&mut read_2);
        assert_eq!(read_2, [0u32]);
        buf.write(&data);
        assert_eq!(buf.len(), 14);
        assert_eq!(buf.capacity().get(), 14);
        assert_eq!(
            buf.buffer.raw_data(),
            [1, 2, 3, 1, 2, 3, 3, 5, 2, 0, 1, 2, 3, 0]
        );

        buf.read_into(&mut read);
        buf.read_into(&mut read);
        buf.read_into(&mut read);
        assert_eq!(read, [2, 3, 0, 1]);
        assert_eq!(buf.read_into(&mut read), 2);
        assert_eq!(read, [2, 3, 0, 1]);

        buf.clear();
        assert_eq!(buf.len(), 0);
        assert_eq!(buf.capacity().get(), 14);

        buf.clear_and_shrink_to_capacity(NonZeroUsize::new(5).unwrap());
        assert_eq!(buf.capacity().get(), 5);
        assert!(buf.raw_capacity().get() >= 5);
    }
}
