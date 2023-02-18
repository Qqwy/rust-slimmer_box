# slimmer_box &emsp; [![Latest Version]][crates.io] [![License]][license path] 

<!-- [![requires: rustc 1.47+]][Rust 1.47] -->


[Latest Version]: https://img.shields.io/crates/v/slimmer_box.svg
[crates.io]: https://crates.io/crates/slimmer_box
[License]: https://img.shields.io/badge/license-MIT-blue.svg
[license path]: https://github.com/djkoloski/slimmer_box/blob/main/LICENSE
[requires: rustc 1.47+]: https://img.shields.io/badge/rustc-1.47+-lightgray.svg
<!-- [Rust 1.47]: https://blog.rust-lang.org/2020/10/08/Rust-1.47.html -->


A SlimmerBox&lt;T> is a packed alternative to Box&lt;T> whose 'fat' pointer is 'slimmer'

## Rationale

A normal `Box<[T]>` is an owned 'fat pointer' that contains both the 'raw' pointer to memory
as well as the size (as an usize) of the managed slice.

On 64-bit targets (where sizeof(usize) == sizeof(u64)), this makes a `Box<[T]>` take up 16 bytes (128 bits, 2 words).
That's a shame: It means that if you build an enum that contains a `Box<[T]>`,
then it will at least require 24 bytes (196 bits, 3 words) of stack memory.

But it is rather common to work with slices that will never be that large.
For example, what if we store the size in a u32 instead?
Will your slices really contain more than 2ˆ32 (4_294_967_296) elements?
a `[u8; 2^32]` takes 4GiB of space.

And since the length is counted in elements, a `[u64; 2^32]` takes 32GiB.

So lets slim this 'fat' pointer down!
By storing the length inside a u32 rather than a u64,
a SlimmerBox<[T], u32> only takes up 12 bytes (96 bits, 1.5 words) rather than 16 bytes.

This allows it to be used inside another structure, such as in one or more variants of an enum.
The resulting structure will then still only take up 16 bytes.

In situations where you are trying to optimize for memory usage, cache locality, etc,
this might make a difference.

## Different sizes

SlimmerBox<T, u32> is the most common version, and therefore u32 is the default SlimmerMetadata to use.
But it is possible to use another variant, if you are sure that your data will be even shorter.

- SlimmerMetadata = `()` is used for sized types. In this case a SlimmerBox will only contain the normal pointer and be exactly 1 word size, just like a normal Box.
- SlimmerMetadata = u64 would make SlimmerBox behave exactly like a normal Box on a 64-bit system.

| SlimmerMetadata | max DST length¹      | resulting size (32bit) | resulting size (64bit) | Notes                                                                           |
|-----------------|----------------------|------------------------|------------------------|---------------------------------------------------------------------------------|
| ()              | -                    | 4 bytes                | 8 bytes                | Used for normal sized types. Identical in size to a normal Box<T> in this case. |
| u8              | 15                   | 5 bytes                | 9 bytes                |                                                                                 |
| u16             | 65535                | 6 bytes                | 10 bytes               | Identical to Box<DST> on 16-bit systems                                         |
| u32             | 4294967295           | 8 bytes (2 words)      | 12 bytes               | Identical to Box<DST> on 32-bit systems                                         |
| u64             | 18446744073709551615 | -²                     | 16 bytes (2 words)     | Identical to Box<DST> on 64-bit systems                                         |

- ¹ Max DST length is in bytes for `str` and in the number of elements for slices.

### Niche optimization

Just like a normal Box, `sizeof(Option<SlimmerBox<T>>) == sizeof(SlimmerBox<T>)`.

## Rkyv

rkyv's Archive, Serialize and Deserialize have been implemented for SlimmerBox.
The serialized version of a SlimmerBox<T> is 'just' a normal `rkyv::ArchivedBox<[T]>`.
This is a match made in heaven, since rkyv's relative pointers use only 32 bits for the pointer part _as well as_ the length part.
As such, `sizeof(rkyv::Archived<SlimmerBox<T>>) == 8` bytes (!).
(This is assuming rkyv's feature `size_32` is used which is the default.
Changing it to `size_64` is rarely useful for the same reason as the rant about lengths above.)

## Limitations

You can _not_ use a SlimmerBox to store a trait object.
This is because the metadata of a `dyn` pointer is another full-sized pointer. We cannot make that smaller!

## `no_std` support

SlimmerBox works perfectly fine in `no_std` environments, as long as the `alloc` crate is available.

(The only thing that is missing in no_std environments are implementations for SlimmerPointee of `std::ffi::OsStr` and `std::ffi::CStr`, neither of which exists when `std` is disabled.)

## Examples

Smaller than a normal Box for dynamically-sized types like slices or strings:

```rust
let array: [u64; 4] = [1, 2, 3, 4];

let slimmer_boxed_slice: SlimmerBox<[u64]> = SlimmerBox::new(&array);
assert_eq!(core::mem::size_of_val(&slimmer_boxed_slice), 12);

let boxed_slice = SlimmerBox::into_box(slimmer_boxed_slice);
assert_eq!(core::mem::size_of_val(&boxed_slice), 16);
```

Works just like normal Box for Sized types:
```
let slimmer_boxed_int: SlimmerBox<u64, ()> = SlimmerBox::new(&42);
assert_eq!(core::mem::size_of_val(&slimmer_boxed_int), 8);

let boxed_int = SlimmerBox::into_box(slimmer_boxed_int);
assert_eq!(core::mem::size_of_val(&boxed_int), 8);
```
