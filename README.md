# slimmer_box &emsp; [![Latest Version]][crates.io] [![License]][license path] 

<!-- [![requires: rustc 1.47+]][Rust 1.47] -->


[Latest Version]: https://img.shields.io/crates/v/slimmer_box.svg
[crates.io]: https://crates.io/crates/slimmer_box
[License]: https://img.shields.io/badge/license-MIT-blue.svg
[license path]: https://github.com/qqwy/rust-slimmer_box/blob/main/LICENSE
[requires: rustc 1.47+]: https://img.shields.io/badge/rustc-1.47+-lightgray.svg
<!-- [Rust 1.47]: https://blog.rust-lang.org/2020/10/08/Rust-1.47.html -->


A [SlimmerBox&lt;T>](https://docs.rs/slimmer_box/0.5.1/slimmer_box/struct.SlimmerBox.html) is a packed alternative to Box&lt;T> whose 'fat' pointer is 'slimmer'

[Documentation](https://docs.rs/slimmer_box/0.5.1/slimmer_box/)

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

- SlimmerMetadata = `()` is used for sized types. In this case a SlimmerBox will only contain the normal pointer and be exactly 1 word size, just like a normal Box containing a sized type.
- SlimmerMetadata = u64 would make SlimmerBox behave exactly like a normal Box containing a dynamically-sized type on a 64-bit system.

| SlimmerMetadata | max DST length¹      | resulting size (32bit) | resulting size (64bit) | Notes                                                                           |
|-----------------|----------------------|------------------------|------------------------|---------------------------------------------------------------------------------|
| ()              | -                    | 4 bytes                | 8 bytes                | Used for normal sized types. Identical in size to a normal Box<T> in this case. |
| u8              | 255                  | 5 bytes                | 9 bytes                |                                                                                 |
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
_(Below examples assume a 64-bit system)_

Smaller than a normal Box for dynamically-sized types like slices or strings:

```rust
use slimmer_box::SlimmerBox;

let array: [u64; 4] = [1, 2, 3, 4];

let boxed_slice: Box<[u64]> = Box::from(&array[..]);
assert_eq!(core::mem::size_of_val(&boxed_slice), 16);

let slimmer_boxed_slice: SlimmerBox<[u64]> = SlimmerBox::new(&array[..]);
assert_eq!(core::mem::size_of_val(&slimmer_boxed_slice), 12);
```

Just like normal Box for normal, Sized types:
```rust
use slimmer_box::SlimmerBox;

let int = 42;

let boxed_int = Box::new(&int);
assert_eq!(core::mem::size_of_val(&boxed_int), 8);

let slimmer_boxed_int: SlimmerBox<u64, ()> = SlimmerBox::new(&int);
assert_eq!(core::mem::size_of_val(&slimmer_boxed_int), 8);

```

You can configure how much space you want to use for the length of a dynamically-sized slice or str:

```rust
use slimmer_box::SlimmerBox;

let array: [u64; 4] = [1, 2, 3, 4];
// Holds at most 255 elements:
let tiny: SlimmerBox<[u64], u8>  = SlimmerBox::new(&array);
assert_eq!(core::mem::size_of_val(&tiny), 9);

// Holds at most 65535 elements or a str of 64kb:
let small: SlimmerBox<[u64], u16>  = SlimmerBox::new(&array);
assert_eq!(core::mem::size_of_val(&small), 10);

// Holds at most 4294967295 elements or a str of 4GB:
let medium: SlimmerBox<[u64], u32>  = SlimmerBox::new(&array);
assert_eq!(core::mem::size_of_val(&medium), 12);

// Holds at most 18446744073709551615 elements, or a str of 16EiB:
let large: SlimmerBox<[u64], u64>  = SlimmerBox::new(&array); // <- Indistinguishable from a normal Box
assert_eq!(core::mem::size_of_val(&large), 16);
```

You can turn a Box into a SlimmerBox and vice-versa:
```rust
use slimmer_box::SlimmerBox;

let message = "hello, world!";
let boxed = Box::new(message);
let slimmer_box = SlimmerBox::from_box(boxed);
let again_boxed = SlimmerBox::into_box(slimmer_box);
```

# Feature flags

- `"std"`. Enabled by default. Disable the default features to use the crate in no_std environments. `slimmer_box` *does* require the `alloc` crate to be available.
- `"rkyv"`. Enable support for the [rkyv](https://crates.io/crates/rkyv) zero-copy serialisation/deserialisation library, which is a very good match for this crate!
- `"serde"`. Enable support for the [serde](https://crates.io/crates/serde) serialisation/deserialisation library.

## MSRV

The minimum supported Rust version of `slimmer_box` is 1.58.1.
