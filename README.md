# Expanding Slice Ring Buffer
![Test](https://github.com/BillyDM/expanding_slice_rb/workflows/Test/badge.svg)
[![Documentation](https://docs.rs/expanding_slice_rb/badge.svg)][documentation]
[![Crates.io](https://img.shields.io/crates/v/expanding_slice_rb.svg)](https://crates.io/crates/expanding_slice_rb)
[![License](https://img.shields.io/crates/l/expanding_slice_rb.svg)](https://github.com/BillyDM/expanding_slice_rb/blob/master/LICENSE)

A self-expanding ring buffer optimized for working with slices of data. Copies/reads with slices are implemented with memcpy. This algorithm attempts to use as little memcpys and allocations as possible, and only potentially shuffles data around when the capacity of the buffer is increased.

This crate is especially useful when working with streams of data where the inputs and outputs use different sizes of buffers.

This buffer cannot be shared across threads, but it could be used as a building block for a ring buffer that does.

## Installation
Add `expanding_slice_rb` as a dependency in your `Cargo.toml`:
```toml
expanding_slice_rb = 0.1
```

## Example
```rust
use expanding_slice_rb::ExpSliceRB;

// Create a ring buffer with type u32. There is no data in the buffer to start.
//
// If possible, it is a good idea to set `capacity` to the largest you expect the
// buffer to get to avoid future memory allocations.
let mut buf = ExpSliceRB::<u32>::with_capacity(3);

let data = [0u32, 1, 2];

// Memcpy data from a slice into the ring buffer. The buffer will automatically
// expand to fill new data.
buf.write(&data);
assert_eq!(buf.len(), 3);
assert_eq!(buf.capacity(), 3);

buf.write(&data);
assert_eq!(buf.len(), 6);
assert_eq!(buf.capacity(), 6);

// Memcpy the next chunk of data into the read slice. If the length of existing
// data in the buffer is less than the length of the slice, then only that amount
// of data will be copied into the front of the slice.
//
// This is streaming, meaning the copied data will be cleared from the buffer for
// reuse, and the next call to `read_into()` will start copying from where the
// previous call left off. If you don't want this behavior, use the `peek_into()`
// method.
let mut read_slice = [5u32; 4];
let mut amount_written = buf.read_into(&mut read_slice);
assert_eq!(amount_written, 4);
assert_eq!(read_slice, [0u32, 1, 2, 0]);

buf.write(&data);
let mut large_read_slice = [5u32; 8];
amount_written = buf.read_into(&mut large_read_slice);
assert_eq!(amount_written, 5);
assert_eq!(large_read_slice, [1u32, 2, 0, 1, 2, 5, 5, 5]);
```

[documentation]: https://docs.rs/expanding_slice_rb/