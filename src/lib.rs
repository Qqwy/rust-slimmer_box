#![feature(rustc_attrs)]
use std::{mem::ManuallyDrop, ptr::NonNull, marker::PhantomData, ops::{Deref, DerefMut}};
use ptr_meta::{Pointee, PtrExt};

use bytecheck::CheckBytes;
use rkyv::{Archive, Deserialize, Serialize, Archived, Resolver, ArchiveUnsized, boxed::ArchivedBox, SerializeUnsized};



use trace::trace;
trace::init_depth_var!();


// #[derive(Archive, Serialize, Deserialize)]
// // #[rustc_layout(debug)]
// // #[archive_attr(rustc_layout(debug))]
// pub enum Foo {
//     First(bool),
//     Second(u32),
//     OutOfLine(SlimmerBox<i32>),
// }

/// A packed alternative to `Box<[T]>` for slices with at most 2ˆ32 (4_294_967_296) elements.
///
/// A normal `Box<[T]>` is an owned 'fat pointer' that contains both the 'raw' pointer to memory
/// as well as the size (as an usize) of the managed slice.
///
/// On 64-bit targets (where sizeof(usize) == sizeof(u64)), this makes a `Box<[T]>` take up 16 bytes (128 bits, 2 words).
/// That's a shame: It means that if you build an enum that contains a `Box<[T]>`,
/// then it will at least require 24 bytes (196 bits, 3 words) of stack memory.
///
/// But it is rather common to work with slices that will never be that large:
/// a `[u8; 2^32]` takes 4GiB of space. Are you really working with strings that are larger in your app?
///
/// And since the length is counted in elements, a `[u64; 2^32]` takes 32GiB.
///
/// By storing the length of such a 'fat pointer' inside a u32 rather than a u64,
/// a SlimmerBox only takes up 12 bytes (96 bits, 1.5 words) rather than 16 bytes.
///
/// This allows it to be used inside another structure, such as in one or more variants of an enum.
/// The resulting structure will then still only take up 16 bytes.
///
/// In situations where you are trying to optimize for memory usage, cache locality, etc,
/// this might make a difference.
///
/// # Niche optimization
/// Just like a normal Box, `sizeof(Option<SlimmerBox<T>>) == sizeof(SlimmerBox<T>)`.
///
/// # Rkyv
/// rkyv's Archive, Serialize and Deserialize have been implemented for SlimmerBox.
/// The serialized version of a SlimmerBox<T> is 'just' a normal `rkyv::ArchivedBox<[T]>`.
/// This is a match made in heaven, since rkyv's relative pointers use only 32 bits for the pointer part _as well as_ the length part.
/// As such, `sizeof(rkyv::Archived<SlimmerBox<T>>) == 8` bytes (!).
/// (This is assuming rkyv's feature `size_32` is used which is the default.
/// Changing it to `size_64` is rarely useful for the same reason as the rant about lengths above.)

#[repr(packed)]
pub struct SlimmerBox<T, SlimMetadata = u32>
where
    T: ?Sized,
    T: SlimPointee<SlimMetadata>,
    SlimMetadata: TryFrom<<T as Pointee>::Metadata> + TryInto<<T as Pointee>::Metadata> + Copy
{
    ptr: core::ptr::NonNull<()>,
    meta: SlimMetadata,
    marker: PhantomData<T>,
}

pub struct PointerMetadataDoesNotFitError<T, SlimMetadata>
where
    T: ?Sized,
    T: SlimPointee<SlimMetadata>,
    SlimMetadata: TryFrom<<T as Pointee>::Metadata> + TryInto<<T as Pointee>::Metadata>
{
    meta: <T as Pointee>::Metadata,
    marker: PhantomData<SlimMetadata>,
}

impl<T, SlimMetadata> std::fmt::Debug for PointerMetadataDoesNotFitError<T, SlimMetadata>
where
    T: ?Sized,
    T: SlimPointee<SlimMetadata>,
    SlimMetadata: TryFrom<<T as Pointee>::Metadata> + TryInto<<T as Pointee>::Metadata>,
    // <T as Pointee>::Metadata: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        f.debug_struct("PointerMetadataDoesNotFitError")
            // .field("meta", &self.meta)
            .finish()
    }
}

impl<T, SlimMetadata> std::fmt::Display for PointerMetadataDoesNotFitError<T, SlimMetadata>
    where
    T: ?Sized,
    T: SlimPointee<SlimMetadata>,
    SlimMetadata: TryFrom<<T as Pointee>::Metadata> + TryInto<<T as Pointee>::Metadata>,
    // <T as Pointee>::Metadata: std::fmt::Debug,
{
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "Pointer Metadata {} ({} bytes) could not be converted to {} ({} bytes)",
               core::any::type_name::<<T as Pointee>::Metadata>(),
               core::mem::size_of::<<T as Pointee>::Metadata>(),
               core::any::type_name::<SlimMetadata>(),
               core::mem::size_of::<SlimMetadata>()
        )
    }
}

/// Generalization of Clone which supports dynamically-sized or otherwise unsized types.
///
/// Implemented for:
/// - [T] as long as T itself is Clone.
/// - `str`.
/// - Any type that itself implements Clone.
///
/// This trait can easily be implemented for any other dynamically-sized type as well.
pub trait CloneUnsized {

    /// Mutates `self` to become a clone of `source`.
    ///
    /// Signature closely matches `std::slice::clone_from_slice()` on purpose.
    fn unsized_clone_from(&mut self, source: &Self);
}

impl<T> CloneUnsized for [T]
where
T: Clone
{
    fn unsized_clone_from(&mut self, source: &Self) {
        self.clone_from_slice(source)
    }
}

impl CloneUnsized for str
{
    fn unsized_clone_from(&mut self, source: &Self) {
        // SAFETY: Cloning valid UTF8 bytes will result in valid UTF8 bytes
        unsafe { self.as_bytes_mut() }.clone_from_slice(source.as_bytes())
    }
}

/// Blanket implementation for any sized T that uses the normal Clone.
impl<T: Clone> CloneUnsized for T {
    fn unsized_clone_from(&mut self, source: &Self) {
        *self = source.clone();
    }
}

/// Trait which can be implemented by any pointer-like types,
/// as long as their metadata might be made smaller.
///
/// As such, it is implemented for:
/// - Zero-sized types (no metadata and actually also no pointer)
/// - Statically-sized types (no metadata)
/// - Dynamically-sized types (metadata indicates length which can be made smaller if the length fits in the smaller int)
///
/// It is _not_ implemented for trait objects, because their metadata is itself a pointer!
///
/// # SlimMetadata bounds
///
/// Note that while there is only a TryInto<Metadata> bound on SlimMetadata,
/// this is only because the compiler does not know that the SlimMetadata values we create
/// only ever originate from Metadata.
/// In other words:
/// `Metadata.try_into::<SlimMetadata>().try_into::<Metadata>().unwrap()` should never fail.
pub trait SlimPointee<SlimMetadata>: Pointee
    where
    <Self as Pointee>::Metadata: Clone,
    SlimMetadata: TryFrom<<Self as Pointee>::Metadata> + TryInto<<Self as Pointee>::Metadata>
{
}

// #[cfg(any(target_pointer_width = "16", target_pointer_width = "32", target_pointer_width = "64"))]
// impl<T> SlimPointee<u16> for [T] {}

#[cfg(any(target_pointer_width = "32", target_pointer_width = "64"))]
impl<T> SlimPointee<u32> for [T] {}

#[cfg(any(target_pointer_width = "64"))]
impl<T> SlimPointee<u64> for [T] {}

#[cfg(any(target_pointer_width = "16", target_pointer_width = "32", target_pointer_width = "64"))]
impl SlimPointee<u16> for str {}

#[cfg(any(target_pointer_width = "32", target_pointer_width = "64"))]
impl SlimPointee<u32> for str {}

#[cfg(any(target_pointer_width = "64"))]
impl SlimPointee<u64> for str {}

#[cfg(any(target_pointer_width = "64"))]
impl<T: Sized> SlimPointee<()> for T {}


impl<T, SlimMetadata> SlimmerBox<T, SlimMetadata>
where
    T: ?Sized,
    T: SlimPointee<SlimMetadata>,
    SlimMetadata: TryFrom<<T as Pointee>::Metadata> + TryInto<<T as Pointee>::Metadata> + Copy,
{

    /// Creates a new SlimmerBox from the given slice.
    ///
    /// This involves cloning the slice (which will clone all elements one by one)
    /// first into a vector, which is then turned into the SlimmerBox.
    ///
    /// Panics if the slice's length cannot fit in a u32.
    pub fn new(slice: &T) -> Self
    where
        T: CloneUnsized,
    {
        Self::try_new(slice).unwrap()
    }

    /// Creates a new SlimmerBox from the given slice.
    ///
    /// This involves cloning the slice (which will clone all elements one by one)
    /// first into a vector, which is then turned into the SlimmerBox.
    ///
    /// # Safety
    /// The caller must ensure that `slice`'s length can fit in a u32.
    pub unsafe fn new_unchecked(slice: &T) -> Self
    where
        T: CloneUnsized,
    {
        Self::try_new(slice).unwrap_unchecked()
    }

    /// Variant of `new` which will return an error if the slice is too long instead of panicing.
    pub fn try_new(slice: &T) -> Result<Self, PointerMetadataDoesNotFitError<T, SlimMetadata>>
    where
        T: CloneUnsized,
    {
        let meta = ptr_meta::metadata(slice);
        let layout = std::alloc::Layout::for_value(slice);
        let alloc_ptr = unsafe { std::alloc::alloc(layout) } as *mut ();
        let target_ptr: *mut T = ptr_meta::from_raw_parts_mut(alloc_ptr, meta);
        // SAFETY: We obtain a reference to newly allocated space
        // This is not yet a valid T, but we only use it to immediately write into
        unsafe { &mut *target_ptr }.unsized_clone_from(slice);

        // SAFETY: We pass a newly allocated, filled, ptr
        unsafe { Self::try_from_raw(target_ptr) }
    }

    /// Variant of `from_box` which will return an error if the slice is too long instead of panicing.
    pub fn try_from_box(boxed: Box<T>) -> Result<Self, PointerMetadataDoesNotFitError<T, SlimMetadata>> {
        let fat_ptr = Box::into_raw(boxed);
        // SAFETY: Box ensures fat_ptr is non-null
        unsafe { Self::try_from_raw(fat_ptr) }
    }

    /// Builds a new SlimmerBox from a raw mutable pointer
    ///
    ///
    /// Panics if the slice's length cannot fit in a u32.
    ///
    /// # Safety
    /// Caller must ensure *T is valid, and non-null
    ///
    /// Furthermore, similar caveats apply as with Box::from_raw.
    pub unsafe fn from_raw(target_ptr: *mut T) -> Self {
        Self::try_from_raw(target_ptr).unwrap()
    }

    /// Builds a new SlimmerBox from a raw mutable pointer
    ///

    /// Variant of `from_box` which will return an error if the slice is too long instead of panicing.
    pub unsafe fn try_from_raw(target_ptr: *mut T) -> Result<Self, PointerMetadataDoesNotFitError<T, SlimMetadata>> {
        let (thin_ptr, meta) = ptr_meta::PtrExt::to_raw_parts(target_ptr);
        let slim_meta = meta.try_into().map_err(|_| PointerMetadataDoesNotFitError{meta, marker: PhantomData})?;

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
    /// This is a fast constant-time operation that needs no allocation.
    ///
    /// Panics if the slice's length cannot fit in a u32.
    pub fn from_box(boxed: Box<T>) -> Self {
        Self::try_from_box(boxed).unwrap()
    }

    /// Variant of `from_box` that will not check whether the slice is short enough.
    ///
    /// # Safety
    /// - The caller needs to ensure that the slice never has more elements than can fit in a u32.
    pub unsafe fn from_box_unchecked(boxed: Box<T>) -> Self {
        Self::try_from_box(boxed).unwrap_unchecked()
    }

    /// Turns a SlimmerBox into a box.
    ///
    /// This is a fast constant-time operation that needs no allocation.
    ///
    /// Not an associated function to not interfere with Deref
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
    pub fn to_ptr(this: &Self) -> *const T {
        let ptr = ptr_meta::from_raw_parts(this.ptr.as_ptr(), SlimmerBox::metadata(this));
        ptr
    }

    /// Turns the SlimmerBox into a raw pointer
    ///
    /// The resulting pointer is guaranteed to be a valid instance of T and non-null.
    ///
    /// Calling this function is safe, but most operations on the result are not.
    /// Similar caveats apply as to Box::into_raw.
    pub fn into_raw(this: Self) -> *mut T {
        let ptr = ptr_meta::from_raw_parts_mut(this.ptr.as_ptr(), SlimmerBox::metadata(&this));
        // Make sure the pointer remains valid; Caller is now responsible for managing the memory:
        core::mem::forget(this);
        ptr
    }

    pub fn slim_metadata(this: &Self) -> SlimMetadata {
        this.meta
    }

    pub fn metadata(this: &Self) -> <T as Pointee>::Metadata {
        let aligned_len = SlimmerBox::slim_metadata(this);
        unsafe { aligned_len.try_into().unwrap_unchecked() }
    }
}

impl<T, SlimMetadata> Drop for SlimmerBox<T, SlimMetadata>
where
    T: ?Sized,
    T: SlimPointee<SlimMetadata>,
    SlimMetadata: TryFrom<<T as Pointee>::Metadata> + TryInto<<T as Pointee>::Metadata> + Copy
{
    fn drop(&mut self) {
        let me = std::mem::replace(self, SlimmerBox { ptr: NonNull::dangling(), meta: self.meta, marker: PhantomData});
        core::mem::forget(self);
        let _drop_this_box = SlimmerBox::into_box(me);
    }
}

impl<T, SlimMetadata> Deref for SlimmerBox<T, SlimMetadata>
where
    T: ?Sized,
    T: SlimPointee<SlimMetadata>,
    SlimMetadata: TryFrom<<T as Pointee>::Metadata> + TryInto<<T as Pointee>::Metadata> + Copy,
{
    type Target = T;
    fn deref(&self) -> &Self::Target {
        let ptr = ptr_meta::from_raw_parts(self.ptr.as_ptr(), SlimmerBox::metadata(self));
        // SAFETY: Correct by construction
        unsafe { &*ptr }
    }
}

impl<T, SlimMetadata> DerefMut for SlimmerBox<T, SlimMetadata>
where
    T: ?Sized,
    T: SlimPointee<SlimMetadata>,
    SlimMetadata: TryFrom<<T as Pointee>::Metadata> + TryInto<<T as Pointee>::Metadata> + Copy,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        let ptr = ptr_meta::from_raw_parts_mut(self.ptr.as_ptr(), SlimmerBox::metadata(self));
        // SAFETY: Correct by construction
        unsafe { &mut  *ptr }
    }
}

impl<T, SlimMetadata> core::borrow::Borrow<T> for SlimmerBox<T, SlimMetadata>
where
    T: ?Sized,
    T: SlimPointee<SlimMetadata>,
    SlimMetadata: TryFrom<<T as Pointee>::Metadata> + TryInto<<T as Pointee>::Metadata> + Copy,
{
    fn borrow(&self) -> &T {
        &**self
    }
}

impl<T, SlimMetadata> core::borrow::BorrowMut<T> for SlimmerBox<T, SlimMetadata>
where
    T: ?Sized,
    T: SlimPointee<SlimMetadata>,
    SlimMetadata: TryFrom<<T as Pointee>::Metadata> + TryInto<<T as Pointee>::Metadata> + Copy,
{
    fn borrow_mut(&mut self) -> &mut T {
        &mut **self
    }
}

impl<T, SlimMetadata> AsRef<T> for SlimmerBox<T, SlimMetadata>
where
    T: ?Sized,
    T: SlimPointee<SlimMetadata>,
    SlimMetadata: TryFrom<<T as Pointee>::Metadata> + TryInto<<T as Pointee>::Metadata> + Copy,
{
    fn as_ref(&self) -> &T {
        &**self
    }
}


impl<T, SlimMetadata> AsMut<T> for SlimmerBox<T, SlimMetadata>
where
    T: ?Sized,
    T: SlimPointee<SlimMetadata>,
    SlimMetadata: TryFrom<<T as Pointee>::Metadata> + TryInto<<T as Pointee>::Metadata> + Copy,
{
    fn as_mut(&mut self) -> &mut T {
        &mut **self
    }
}

impl<T, SlimMetadata> Unpin for SlimmerBox<T, SlimMetadata>
where
    T: ?Sized,
    T: SlimPointee<SlimMetadata>,
    SlimMetadata: TryFrom<<T as Pointee>::Metadata> + TryInto<<T as Pointee>::Metadata> + Copy,
{}

impl<T, SlimMetadata> Clone for SlimmerBox<T, SlimMetadata>
where
    T: Clone,
    T: ?Sized,
    T: SlimPointee<SlimMetadata>,
    SlimMetadata: TryFrom<<T as Pointee>::Metadata> + TryInto<<T as Pointee>::Metadata> + Copy,
{
    fn clone(&self) -> Self {
        let slice = self.deref();
        // SAFETY: The original SlimmerBox already checked this invariant on construction
        unsafe { SlimmerBox::new_unchecked(slice) }
    }
}

impl<T, SlimMetadata> core::fmt::Debug for SlimmerBox<T, SlimMetadata>
where
    T: core::fmt::Debug,
    T: ?Sized,
    T: SlimPointee<SlimMetadata>,
    SlimMetadata: TryFrom<<T as Pointee>::Metadata> + TryInto<<T as Pointee>::Metadata> + Copy,
{
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Debug::fmt(&**self, f)
    }
}

// #[derive(Debug, Clone, Archive, Serialize, Deserialize)]
// #[archive_attr(derive(CheckBytes))]
// #[archive_attr(derive(Debug))]
// #[archive(compare(PartialEq, PartialOrd))]
// #[archive_attr(rustc_layout(debug))]
// #[rustc_layout(debug)]
pub enum Thing{
    LocalString{bytes: [u8; 14], len: u8},
    RemoteString{ptr: SlimmerBox<str>},
}

// #[rustc_layout(debug)]
// #[repr(transparent)]
// pub struct Foo {
//     ptr: Box<str>,
// }

// pub const FANOUT: usize = 10;
// pub type NodeId = usize;

// #[derive(Debug, Clone, Archive, Serialize, Deserialize)]
// pub enum SerializableNode<K, V>
// where
//     K: Default + rkyv::Archive,
//     V: Default + rkyv::Archive,
// {
//     Leaf {
//         keys: tinyvec::ArrayVec<[K; FANOUT]>,
//         values: tinyvec::ArrayVec<[V; FANOUT]>,
//     },
//     Internal {
// 	#[omit_bounds]
//         keys: tinyvec::ArrayVec<[K; FANOUT]>,
//         children: tinyvec::ArrayVec<[NodeId; FANOUT]>,
//     },
// }

// #[derive(Debug, Clone, Archive, Serialize, Deserialize)]
// #[archive_attr(derive(CheckBytes))]
// // #[archive_attr(derive(Debug))] // <-- This line is problematic!
// pub struct Wrapper<T> {
//     stuff: T,
// }

// // use std::fmt::Debug;
// // impl<T> Debug for ArchivedWrapper<T>
// // where
// //     T: Archive,
// //     <T as Archive>::Archived: Debug,
// // {
// //     fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
// //         f.debug_struct("Wrapper")
// //             .field("stuff", &self.stuff)
// //             .finish()
// //     }
// // }



// pub struct SlimmerBoxResolver<T>(rkyv::boxed::BoxResolver<<[T] as ArchiveUnsized>::MetadataResolver>)
// where
//     Box<[T]>: Archive,
//     [T]: ArchiveUnsized;

// /// SlimmerBox is archived into an ArchivedBox<[T]>, just like a normal box.
// impl<T> Archive for SlimmerBox<T>
//     where
//     Box<[T]>: Archive,
//     [T]: ArchiveUnsized,
// {
//     type Archived = ArchivedBox<<[T] as ArchiveUnsized>::Archived>;
//     type Resolver = SlimmerBoxResolver<T>;

//     #[inline]
//     unsafe fn resolve(&self, pos: usize, resolver: Self::Resolver, out: *mut Self::Archived) {
//         println!("Resolving {:?}", &self as *const _);
//         rkyv::boxed::ArchivedBox::resolve_from_ref(self.as_ref(), pos, resolver.0, out)
//     }
// }

// impl<S: rkyv::Fallible + ?Sized, T> Serialize<S> for SlimmerBox<T>
//     where
//     Box<[T]>: Serialize<S>,
//     [T]: SerializeUnsized<S>,
// {
//     #[inline]
//     fn serialize(&self, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
//         println!("Serializing {:?}", &self as *const _);
//         let res = ArchivedBox::serialize_from_ref(self.as_ref(), serializer)?;
//         Ok(SlimmerBoxResolver(res))
//     }
// }

// impl<T, D> Deserialize<SlimmerBox<T>, D> for ArchivedBox<<[T] as ArchiveUnsized>::Archived>
//     where
//     [T]: ArchiveUnsized,
//     <[T] as ArchiveUnsized>::Archived: rkyv::DeserializeUnsized<[T], D>,
//     D: rkyv::Fallible + ?Sized,
// {
//     fn deserialize(&self, deserializer: &mut D) -> Result<SlimmerBox<T>, D::Error> {
//         println!("Deserializing {:?}", &self as *const _);
//         let boxed: Box<[T]> = self.deserialize(deserializer)?;
//         Ok(SlimmerBox::from_box(boxed))
//     }
// }


#[cfg(test)]
mod tests {
    use crate::SlimmerBox;

    #[test]
    fn roundtrip() {
        let slice: [u64; 4] = [1,2,3,4];
        let boxed: SlimmerBox<_, _> = SlimmerBox::new(&slice);
        println!("slimmerbox (array): {}", core::mem::size_of_val(&boxed));
        println!("slimmerbox (array): {:?}", boxed);
        assert_eq!(core::mem::size_of_val(&boxed), 8);
        let result = SlimmerBox::into_box(boxed);
        println!("       box (array): {}", core::mem::size_of_val(&result));
        println!("       box (array): {:?}", result);
        assert_eq!(core::mem::size_of_val(&result), 8);

        let boxed_slice: SlimmerBox<[u64]> = SlimmerBox::new(&slice);
        println!("slimmerbox (slice): {}", core::mem::size_of_val(&boxed_slice));
        println!("slimmerbox (slice): {:?}", boxed_slice);
        assert_eq!(core::mem::size_of_val(&boxed_slice), 12);

        let result = SlimmerBox::into_box(boxed_slice);
        println!("       box (slice): {}", core::mem::size_of_val(&result));
        println!("       box (slice): {:?}", result);
        assert_eq!(core::mem::size_of_val(&result), 16);
    }


    // #[test]
    // fn rkyv() {
    //     let boxed = SlimmerBox::new(&[1,2,3,4].as_slice());
    //     println!("{:?}", &boxed);
    //     let bytes = rkyv::to_bytes::<_, 64>(&boxed).unwrap();
    //     println!("{:?}", bytes);
    //     let deserialized: SlimmerBox<i32> = unsafe { rkyv::from_bytes_unchecked(&bytes) }.unwrap();
    //     println!("{:?}", deserialized);

    // }
}
