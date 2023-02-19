#![no_std]
//! slimmer_box: A packed alternative to [`Box<T>`](alloc::boxed::Box) whose 'fat' pointer is 'slimmer'.
//!
//!
//! [`SlimmerBox<T>`] is the main type exposed by this crate. Detailed documentation can be found there.
//!
//! Other, less frequently useful types:
//! - [`CloneUnsized`]: a helper trait to also allow unsized types whose _contents_ are clone. (like [`[T]`](slice) and [`str`]) to be cloned around.
//! - [`SlimmerPointee`]: a helper trait implemented for all sized and unsized types for which a 'fat' pointer might be made slimmer.
//!
//! # Feature flags
//!
//! - `"std"`. Enabled by default. Disable the default features to use the crate in no_std environments. `slimmer_box` *does* require the `alloc` crate to be available.
//! - `"rkyv"`. Enable support for the [rkyv](https://crates.io/crates/rkyv) zero-copy serialisation/deserialisation library, which is a very good match for this crate!
//! - `"serde"`. Enable support for the [serde](https://crates.io/crates/serde) serialisation/deserialisation library.
//!
//!
//! # MSRV
//! The minimum supported Rust version of `slimmer_box` is 1.58.1.

// Enable std in tests for easier debugging
#[cfg(any(feature = "std", test))]
#[macro_use]
extern crate std;

extern crate alloc;
use alloc::boxed::Box;

use core::{
    marker::PhantomData,
    ops::{Deref, DerefMut},
    ptr::NonNull,
};
use ptr_meta::Pointee;

pub mod clone_unsized;
pub mod slim_pointee;
pub use crate::clone_unsized::CloneUnsized;
pub use crate::slim_pointee::SlimmerPointee;

#[cfg(feature = "rkyv")]
pub mod rkyv;

#[cfg(feature = "serde")]
pub mod serde;

/// A packed alternative to [`Box<T>`](alloc::boxed::Box) whose 'fat' pointer is 'slimmer'.
///
/// A normal `Box<[T]>` is an owned 'fat pointer' that contains both the 'raw' pointer to memory
/// as well as the size (as an usize) of the managed slice.
///
/// On 64-bit targets (where sizeof(usize) == sizeof(u64)), this makes a `Box<[T]>` take up 16 bytes (128 bits, 2 words).
/// That's a shame: It means that if you build an enum that contains a `Box<[T]>`,
/// then it will at least require 24 bytes (196 bits, 3 words) of stack memory.
///
/// But it is rather common to work with slices that will never be that large.
/// For example, what if we store the size in a u32 instead?
/// Will your slices really contain more than 2ˆ32 (4_294_967_296) elements?
/// a `[u8; 2^32]` takes 4GiB of space.
///
/// And since the length is counted in elements, a `[u64; 2^32]` takes 32GiB.
///
/// So lets slim this 'fat' pointer down!
/// By storing the length inside a u32 rather than a u64,
/// a SlimmerBox<\[T\], u32> only takes up 12 bytes (96 bits, 1.5 words) rather than 16 bytes.
///
/// This allows it to be used inside another structure, such as in one or more variants of an enum.
/// The resulting structure will then still only take up 16 bytes.
///
/// In situations where you are trying to optimize for memory usage, cache locality, etc,
/// this might make a difference.
///
/// # Different sizes
///
/// SlimmerBox<T, u32> is the most common version, and therefore u32 is the default SlimmerMetadata to use.
/// But it is possible to use another variant, if you are sure that your data will be even shorter.
///
/// - SlimmerMetadata = [`()`](unit) is used for sized types. In this case a SlimmerBox will only contain the normal pointer and be exactly 1 word size, just like a normal [Box](alloc::boxed::Box).
/// - SlimmerMetadata = [u64](u64) would make SlimmerBox behave exactly like a normal Box on a 64-bit system.
///
/// | SlimmerMetadata | max DST length¹      | resulting size (32bit) | resulting size (64bit) | Notes                                                                           |
/// |-----------------|----------------------|------------------------|------------------------|---------------------------------------------------------------------------------|
/// | ()              | -                    | 4 bytes                | 8 bytes                | Used for normal sized types. Identical in size to a normal `Box<T>` in this case. |
/// | u8              | 255                  | 5 bytes                | 9 bytes                |                                                                                 |
/// | u16             | 65535                | 6 bytes                | 10 bytes               | Identical to `Box<DST>` on 16-bit systems                                         |
/// | u32             | 4294967295           | 8 bytes (2 words)      | 12 bytes               | Identical to `Box<DST>` on 32-bit systems                                         |
/// | u64             | 18446744073709551615 | -²                     | 16 bytes (2 words)     | Identical to `Box<DST>` on 64-bit systems                                         |
///
/// - ¹ Max DST length is in bytes for `str` and in the number of elements for slices.
///
/// # Niche optimization
///
/// Just like a normal Box, `sizeof(Option<SlimmerBox<T>>) == sizeof(SlimmerBox<T>)`.
///
/// # Rkyv
///
/// rkyv's Archive, Serialize and Deserialize have been implemented for SlimmerBox.
/// The serialized version of a `SlimmerBox<T>` is 'just' a normal [`rkyv::ArchivedBox<[T]>`].
/// This is a match made in heaven, since rkyv's relative pointers use only 32 bits for the pointer part _as well as_ the length part.
/// As such, `sizeof(rkyv::Archived<SlimmerBox<T>>) == 8` bytes (!).
/// (This is assuming rkyv's feature `size_32` is used which is the default.
/// Changing it to `size_64` is rarely useful for the same reason as the rant about lengths above.)
///
/// # Limitations
///
/// You can _not_ use a SlimmerBox to store a trait object.
/// This is because the metadata of a `dyn` pointer is another full-sized pointer. We cannot make that smaller!
///
/// # `no_std` support
///
/// SlimmerBox works perfectly fine in `no_std` environments, as long as the `alloc` crate is available.
///
/// (The only thing that is missing in no_std environments are implementations for [SlimmerPointee](SlimmerPointee) of `std::ffi::OsStr` and `std::ffi::CStr`, neither of which exists when `std` is disabled.)
///
///
/// ## Examples
/// _(Below examples assume a 64-bit system)_
///
/// Smaller than a normal Box for dynamically-sized types like slices or strings:
///
/// ```rust
/// use slimmer_box::SlimmerBox;
///
/// let array: [u64; 4] = [1, 2, 3, 4];
///
/// let boxed_slice: Box<[u64]> = Box::from(&array[..]);
/// assert_eq!(core::mem::size_of_val(&boxed_slice), 16);
///
/// let slimmer_boxed_slice: SlimmerBox<[u64]> = SlimmerBox::new(&array[..]);
/// assert_eq!(core::mem::size_of_val(&slimmer_boxed_slice), 12);
/// ```
///
/// Just like normal Box for normal, Sized types:
/// ```rust
/// use slimmer_box::SlimmerBox;
///
/// let int = 42;
///
/// let boxed_int = Box::new(&int);
/// assert_eq!(core::mem::size_of_val(&boxed_int), 8);
///
/// let slimmer_boxed_int: SlimmerBox<u64, ()> = SlimmerBox::new(&int);
/// assert_eq!(core::mem::size_of_val(&slimmer_boxed_int), 8);
///
/// ```
///
/// You can configure how much space you want to use for the length of a dynamically-sized slice or str:
///
/// ```rust
/// use slimmer_box::SlimmerBox;
///
/// let array: [u64; 4] = [1, 2, 3, 4];
/// // Holds at most 255 elements:
/// let tiny: SlimmerBox<[u64], u8>  = SlimmerBox::new(&array);
/// assert_eq!(core::mem::size_of_val(&tiny), 9);
///
/// // Holds at most 65535 elements or a str of 64kb:
/// let small: SlimmerBox<[u64], u16>  = SlimmerBox::new(&array);
/// assert_eq!(core::mem::size_of_val(&small), 10);
///
/// // Holds at most 4294967295 elements or a str of 4GB:
/// let medium: SlimmerBox<[u64], u32>  = SlimmerBox::new(&array);
/// assert_eq!(core::mem::size_of_val(&medium), 12);
///
/// // Holds at most 18446744073709551615 elements, or a str of 16EiB:
/// let large: SlimmerBox<[u64], u64>  = SlimmerBox::new(&array); // <- Indistinguishable from a normal Box
/// assert_eq!(core::mem::size_of_val(&large), 16);
/// ```
///
/// You can turn a Box into a SlimmerBox and vice-versa:
/// ```rust
/// use slimmer_box::SlimmerBox;
///
/// let message = "hello, world!";
/// let boxed = Box::new(message);
/// let slimmer_box = SlimmerBox::from_box(boxed);
/// let again_boxed = SlimmerBox::into_box(slimmer_box);
/// ```
///
#[repr(packed)]
pub struct SlimmerBox<T, SlimmerMetadata = u32>
where
    T: ?Sized,
    T: SlimmerPointee<SlimmerMetadata>,
    SlimmerMetadata: TryFrom<<T as Pointee>::Metadata> + TryInto<<T as Pointee>::Metadata> + Copy,
{
    ptr: core::ptr::NonNull<()>,
    meta: SlimmerMetadata,
    marker: PhantomData<T>,
}

pub struct PointerMetadataDoesNotFitError<T, SlimmerMetadata>(
    PhantomData<T>,
    PhantomData<SlimmerMetadata>,
)
where
    T: ?Sized,
    T: SlimmerPointee<SlimmerMetadata>,
    SlimmerMetadata: TryFrom<<T as Pointee>::Metadata> + TryInto<<T as Pointee>::Metadata>;

impl<T, SlimmerMetadata> core::fmt::Debug for PointerMetadataDoesNotFitError<T, SlimmerMetadata>
where
    T: ?Sized,
    T: SlimmerPointee<SlimmerMetadata>,
    SlimmerMetadata: TryFrom<<T as Pointee>::Metadata> + TryInto<<T as Pointee>::Metadata>,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> Result<(), core::fmt::Error> {
        f.debug_struct("PointerMetadataDoesNotFitError")
            .field(
                "metadata_type",
                &core::any::type_name::<<T as Pointee>::Metadata>(),
            )
            .field(
                "slimmer_metadata_type",
                &core::any::type_name::<SlimmerMetadata>(),
            )
            .finish()
    }
}

impl<T, SlimmerMetadata> core::fmt::Display for PointerMetadataDoesNotFitError<T, SlimmerMetadata>
where
    T: ?Sized,
    T: SlimmerPointee<SlimmerMetadata>,
    SlimmerMetadata: TryFrom<<T as Pointee>::Metadata> + TryInto<<T as Pointee>::Metadata>,
    // <T as Pointee>::Metadata: core::fmt::Debug,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> Result<(), core::fmt::Error> {
        write!(
            f,
            "Pointer Metadata {} ({} bytes) could not be converted to {} ({} bytes)",
            core::any::type_name::<<T as Pointee>::Metadata>(),
            core::mem::size_of::<<T as Pointee>::Metadata>(),
            core::any::type_name::<SlimmerMetadata>(),
            core::mem::size_of::<SlimmerMetadata>()
        )
    }
}

impl<T, SlimmerMetadata> SlimmerBox<T, SlimmerMetadata>
where
    T: ?Sized,
    T: SlimmerPointee<SlimmerMetadata>,
    SlimmerMetadata: TryFrom<<T as Pointee>::Metadata> + TryInto<<T as Pointee>::Metadata> + Copy,
{
    /// Creates a new SlimmerBox from the given value (which may be a slice, string or other dynamically sized type).
    ///
    /// This involves cloning the slice (which will clone all elements one by one)
    /// and as such only works for types whose contents are cloneable.
    /// Otherwise, use `from_box`.
    ///
    /// Panics if the value's Metadata is too large to fit in SlimmerMetadata.
    pub fn new(value: &T) -> Self
    where
        T: CloneUnsized,
    {
        Self::try_new(value).unwrap()
    }

    /// Variant of `new` that skips its size check.
    ///
    /// # Safety
    /// The caller must ensure that `slice`'s length can fit in a u32.
    pub unsafe fn new_unchecked(value: &T) -> Self
    where
        T: CloneUnsized,
    {
        Self::try_new(value).unwrap_unchecked()
    }

    /// Variant of `new` which will return an error if the slice is too long instead of panicing.
    pub fn try_new(value: &T) -> Result<Self, PointerMetadataDoesNotFitError<T, SlimmerMetadata>>
    where
        T: CloneUnsized,
    {
        let meta = ptr_meta::metadata(value);
        let target_ptr = if core::mem::size_of_val(value) > 0 {
            println!("Normal type");
            // Normally-sized type (or DST with non-empty size):
            let layout = core::alloc::Layout::for_value(value);
            let alloc_ptr = unsafe { alloc::alloc::alloc(layout) } as *mut ();
            let target_ptr: *mut T = ptr_meta::from_raw_parts_mut(alloc_ptr, meta);
            // SAFETY: We obtain a reference to newly allocated space
            // This is not yet a valid T, but we only use it to immediately write into
            unsafe { &mut *target_ptr }.unsized_clone_from(value);
            target_ptr
        } else {
            // ZST, (or DST with zero size like an empty slice)
            // no allocation needed nor desired
            core::ptr::addr_of!(*value) as *mut T
        };

        // SAFETY: We pass a newly allocated, filled, ptr
        unsafe { Self::try_from_raw(target_ptr) }
    }

    /// Variant of `from_box` which will return an error if the value's metadata is too large instead of panicing.
    pub fn try_from_box(
        boxed: Box<T>,
    ) -> Result<Self, PointerMetadataDoesNotFitError<T, SlimmerMetadata>> {
        let fat_ptr = Box::into_raw(boxed);
        // SAFETY: Box ensures fat_ptr is non-null
        unsafe { Self::try_from_raw(fat_ptr) }
    }

    /// Builds a new SlimmerBox from a raw mutable pointer
    ///
    /// Panics if the type's metadata is too large.
    ///
    /// # Safety
    /// Caller must ensure *T is valid, and non-null
    ///
    /// Furthermore, similar caveats apply as with Box::from_raw.
    pub unsafe fn from_raw(target_ptr: *mut T) -> Self {
        Self::try_from_raw(target_ptr).unwrap()
    }

    /// Variant of `from_raw` which will return an error if the value's metadata is too large instead of panicing.
    ///
    /// # Safety
    /// Caller must ensure *T is valid, and non-null
    ///
    /// Furthermore, similar caveats apply as with Box::from_raw.
    pub unsafe fn try_from_raw(
        target_ptr: *mut T,
    ) -> Result<Self, PointerMetadataDoesNotFitError<T, SlimmerMetadata>> {
        let (thin_ptr, meta) = ptr_meta::PtrExt::to_raw_parts(target_ptr);
        let slim_meta = meta
            .try_into()
            .map_err(|_| PointerMetadataDoesNotFitError(PhantomData, PhantomData))?;

        // SAFETY: Box ensures its ptr is never null.
        let ptr = unsafe { core::ptr::NonNull::new_unchecked(thin_ptr as *mut ()) };
        Ok(Self {
            ptr,
            meta: slim_meta,
            marker: PhantomData,
        })
    }

    /// Turns a Box into a SlimmerBox.
    ///
    /// This is a fast constant-time operation that needs no allocation, as it consumes the box.
    ///
    /// Panics if the pointer's metadata is too large to made slimmer.
    pub fn from_box(boxed: Box<T>) -> Self {
        Self::try_from_box(boxed).unwrap()
    }

    /// Variant of `from_box` that will not check whether the slice is short enough.
    ///
    /// # Safety
    /// The caller needs to ensure that the conversion from Metadata to SlimmerMetadata will not fail.
    pub unsafe fn from_box_unchecked(boxed: Box<T>) -> Self {
        Self::try_from_box(boxed).unwrap_unchecked()
    }

    /// Turns a SlimmerBox into a box.
    ///
    /// This is a fast constant-time operation that needs no allocation.
    ///
    /// Not an associated function to not interfere with Deref, so use fully qualified syntax to call it.
    pub fn into_box(this: Self) -> Box<T> {
        let ptr = Self::into_raw(this);
        // SAFETY: We reconstruct using the inverse operations from construction
        unsafe { Box::from_raw(ptr) }
    }

    /// Obtains a raw read-only (non-owned) pointer view of the contents of this SlimmerBox.
    ///
    /// The resulting pointer is guaranteed to be a valid instance of T and non-null.
    ///
    /// This function is mainly useful if you need to implement something that exists for Box
    /// but not (yet) for SlimmerBox. Feel free to open an issue or contribute a PR!
    ///
    /// Not an associated function to not interfere with Deref, so use fully qualified syntax to call it.
    pub fn to_ptr(this: &Self) -> *const T {
        ptr_meta::from_raw_parts(this.ptr.as_ptr(), SlimmerBox::metadata(this))
    }

    /// Turns the SlimmerBox into a raw pointer
    ///
    /// The resulting pointer is guaranteed to be a valid instance of T and non-null.
    ///
    /// Calling this function is safe, but most operations on the result are not.
    /// Similar caveats apply as to Box::into_raw.
    ///
    /// Not an associated function to not interfere with Deref, so use fully qualified syntax to call it.
    pub fn into_raw(this: Self) -> *mut T {
        let ptr = ptr_meta::from_raw_parts_mut(this.ptr.as_ptr(), SlimmerBox::metadata(&this));
        // Make sure the pointer remains valid; Caller is now responsible for managing the memory:
        core::mem::forget(this);
        ptr
    }

    /// Retrieve access to the stored slimmer metadata value.
    ///
    /// Not an associated function to not interfere with Deref, so use fully qualified syntax to call it.
    pub fn slim_metadata(this: &Self) -> SlimmerMetadata {
        this.meta
    }

    /// Returns the outcome of converting the stored SlimmerMetadata value back into its original Metadata form.
    ///
    /// Not an associated function to not interfere with Deref, so use fully qualified syntax to call it.
    pub fn metadata(this: &Self) -> <T as Pointee>::Metadata {
        let aligned_len = SlimmerBox::slim_metadata(this);
        // SAFETY: Guaranteed to not fail by the unsafe SlimmerPointee trait
        unsafe { aligned_len.try_into().unwrap_unchecked() }
    }
}

impl<T, SlimmerMetadata> Drop for SlimmerBox<T, SlimmerMetadata>
where
    T: ?Sized,
    T: SlimmerPointee<SlimmerMetadata>,
    SlimmerMetadata: TryFrom<<T as Pointee>::Metadata> + TryInto<<T as Pointee>::Metadata> + Copy,
{
    fn drop(&mut self) {
        // NOTE: The garbage value will not be dropped which is exactly what we want
        let me = core::mem::replace(
            self,
            SlimmerBox {
                ptr: NonNull::dangling(),
                meta: self.meta,
                marker: PhantomData,
            },
        );
        let _drop_this_box = SlimmerBox::into_box(me);
    }
}

impl<T, SlimmerMetadata> Deref for SlimmerBox<T, SlimmerMetadata>
where
    T: ?Sized,
    T: SlimmerPointee<SlimmerMetadata>,
    SlimmerMetadata: TryFrom<<T as Pointee>::Metadata> + TryInto<<T as Pointee>::Metadata> + Copy,
{
    type Target = T;
    fn deref(&self) -> &Self::Target {
        let ptr = ptr_meta::from_raw_parts(self.ptr.as_ptr(), SlimmerBox::metadata(self));
        // SAFETY: Correct by construction
        unsafe { &*ptr }
    }
}

impl<T, SlimmerMetadata> DerefMut for SlimmerBox<T, SlimmerMetadata>
where
    T: ?Sized,
    T: SlimmerPointee<SlimmerMetadata>,
    SlimmerMetadata: TryFrom<<T as Pointee>::Metadata> + TryInto<<T as Pointee>::Metadata> + Copy,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        let ptr = ptr_meta::from_raw_parts_mut(self.ptr.as_ptr(), SlimmerBox::metadata(self));
        // SAFETY: Correct by construction
        unsafe { &mut *ptr }
    }
}

impl<T, SlimmerMetadata> core::borrow::Borrow<T> for SlimmerBox<T, SlimmerMetadata>
where
    T: ?Sized,
    T: SlimmerPointee<SlimmerMetadata>,
    SlimmerMetadata: TryFrom<<T as Pointee>::Metadata> + TryInto<<T as Pointee>::Metadata> + Copy,
{
    fn borrow(&self) -> &T {
        self
    }
}

impl<T, SlimmerMetadata> core::borrow::BorrowMut<T> for SlimmerBox<T, SlimmerMetadata>
where
    T: ?Sized,
    T: SlimmerPointee<SlimmerMetadata>,
    SlimmerMetadata: TryFrom<<T as Pointee>::Metadata> + TryInto<<T as Pointee>::Metadata> + Copy,
{
    fn borrow_mut(&mut self) -> &mut T {
        self
    }
}

impl<T, SlimmerMetadata> AsRef<T> for SlimmerBox<T, SlimmerMetadata>
where
    T: ?Sized,
    T: SlimmerPointee<SlimmerMetadata>,
    SlimmerMetadata: TryFrom<<T as Pointee>::Metadata> + TryInto<<T as Pointee>::Metadata> + Copy,
{
    fn as_ref(&self) -> &T {
        self
    }
}

impl<T, SlimmerMetadata> AsMut<T> for SlimmerBox<T, SlimmerMetadata>
where
    T: ?Sized,
    T: SlimmerPointee<SlimmerMetadata>,
    SlimmerMetadata: TryFrom<<T as Pointee>::Metadata> + TryInto<<T as Pointee>::Metadata> + Copy,
{
    fn as_mut(&mut self) -> &mut T {
        self
    }
}

impl<T, SlimmerMetadata> Unpin for SlimmerBox<T, SlimmerMetadata>
where
    T: ?Sized,
    T: SlimmerPointee<SlimmerMetadata>,
    SlimmerMetadata: TryFrom<<T as Pointee>::Metadata> + TryInto<<T as Pointee>::Metadata> + Copy,
{
}

impl<T, SlimmerMetadata> Clone for SlimmerBox<T, SlimmerMetadata>
where
    T: Clone,
    T: ?Sized,
    T: SlimmerPointee<SlimmerMetadata>,
    SlimmerMetadata: TryFrom<<T as Pointee>::Metadata> + TryInto<<T as Pointee>::Metadata> + Copy,
{
    fn clone(&self) -> Self {
        let value = self.deref();
        // SAFETY: The original SlimmerBox already checked this invariant on construction
        unsafe { SlimmerBox::new_unchecked(value) }
    }
}

impl<T, SlimmerMetadata> core::fmt::Debug for SlimmerBox<T, SlimmerMetadata>
where
    T: core::fmt::Debug,
    T: ?Sized,
    T: SlimmerPointee<SlimmerMetadata>,
    SlimmerMetadata: TryFrom<<T as Pointee>::Metadata> + TryInto<<T as Pointee>::Metadata> + Copy,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Debug::fmt(&**self, f)
    }
}

impl<T: PartialEq, SlimmerMetadata> PartialEq for SlimmerBox<T, SlimmerMetadata>
where
    T: ?Sized,
    T: SlimmerPointee<SlimmerMetadata>,
    SlimmerMetadata: TryFrom<<T as Pointee>::Metadata> + TryInto<<T as Pointee>::Metadata> + Copy,
{
    #[inline]
    fn eq(&self, other: &Self) -> bool {
        PartialEq::eq(&**self, &**other)
    }
    #[inline]
    fn ne(&self, other: &Self) -> bool {
        PartialEq::ne(&**self, &**other)
    }
}

impl<T: PartialOrd, SlimmerMetadata> PartialOrd for SlimmerBox<T, SlimmerMetadata>
where
    T: ?Sized,
    T: SlimmerPointee<SlimmerMetadata>,
    SlimmerMetadata: TryFrom<<T as Pointee>::Metadata> + TryInto<<T as Pointee>::Metadata> + Copy,
{
    #[inline]
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        PartialOrd::partial_cmp(&**self, &**other)
    }
    #[inline]
    fn lt(&self, other: &Self) -> bool {
        PartialOrd::lt(&**self, &**other)
    }
    #[inline]
    fn le(&self, other: &Self) -> bool {
        PartialOrd::le(&**self, &**other)
    }
    #[inline]
    fn ge(&self, other: &Self) -> bool {
        PartialOrd::ge(&**self, &**other)
    }
    #[inline]
    fn gt(&self, other: &Self) -> bool {
        PartialOrd::gt(&**self, &**other)
    }
}

impl<T: Ord, SlimmerMetadata> Ord for SlimmerBox<T, SlimmerMetadata>
where
    T: ?Sized,
    T: SlimmerPointee<SlimmerMetadata>,
    SlimmerMetadata: TryFrom<<T as Pointee>::Metadata> + TryInto<<T as Pointee>::Metadata> + Copy,
{
    #[inline]
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        Ord::cmp(&**self, &**other)
    }
}
impl<T: Eq, SlimmerMetadata> Eq for SlimmerBox<T, SlimmerMetadata>
where
    T: ?Sized,
    T: SlimmerPointee<SlimmerMetadata>,
    SlimmerMetadata: TryFrom<<T as Pointee>::Metadata> + TryInto<<T as Pointee>::Metadata> + Copy,
{
}

impl<T: core::hash::Hash, SlimmerMetadata> core::hash::Hash for SlimmerBox<T, SlimmerMetadata>
where
    T: ?Sized,
    T: SlimmerPointee<SlimmerMetadata>,
    SlimmerMetadata: TryFrom<<T as Pointee>::Metadata> + TryInto<<T as Pointee>::Metadata> + Copy,
{
    fn hash<H: core::hash::Hasher>(&self, state: &mut H) {
        (**self).hash(state);
    }
}

#[cfg(test)]
mod tests {
    use crate::SlimmerBox;

    #[test]
    fn roundtrip() {
        let slice: [u64; 4] = [1, 2, 3, 4];
        let boxed: SlimmerBox<_, _> = SlimmerBox::new(&slice);
        println!("slimmerbox (array): {}", core::mem::size_of_val(&boxed));
        println!("slimmerbox (array): {boxed:?}");
        assert_eq!(core::mem::size_of_val(&boxed), 8);
        let result = SlimmerBox::into_box(boxed);
        println!("       box (array): {}", core::mem::size_of_val(&result));
        println!("       box (array): {result:?}");
        assert_eq!(core::mem::size_of_val(&result), 8);

        let boxed_slice: SlimmerBox<[u64]> = SlimmerBox::new(&slice);
        println!(
            "slimmerbox (slice): {}",
            core::mem::size_of_val(&boxed_slice)
        );
        println!("slimmerbox (slice): {boxed_slice:?}");
        assert_eq!(core::mem::size_of_val(&boxed_slice), 12);

        let result = SlimmerBox::into_box(boxed_slice);
        println!("       box (slice): {}", core::mem::size_of_val(&result));
        println!("       box (slice): {result:?}");
        assert_eq!(core::mem::size_of_val(&result), 16);
    }

    #[test]
    fn zst() {
        let boxed_unit = SlimmerBox::new(&());
        println!("{:?}", boxed_unit);
        let _unit2 = *SlimmerBox::into_box(boxed_unit).clone();
    }

    #[test]
    fn empty_slice() {
        let boxed_slice: SlimmerBox<[u64]> = SlimmerBox::new(&[]);
        assert_eq!(core::mem::size_of_val(&boxed_slice), 12);
        println!("{:?}", boxed_slice);
    }
}
