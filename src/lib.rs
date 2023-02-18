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
//     OutOfLine(SmallSliceBox<i32>),
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
/// a SmallSliceBox only takes up 12 bytes (96 bits, 1.5 words) rather than 16 bytes.
///
/// This allows it to be used inside another structure, such as in one or more variants of an enum.
/// The resulting structure will then still only take up 16 bytes.
///
/// In situations where you are trying to optimize for memory usage, cache locality, etc,
/// this might make a difference.
///
/// # Niche optimization
/// Just like a normal Box, `sizeof(Option<SmallSliceBox<T>>) == sizeof(SmallSliceBox<T>)`.
///
/// # Rkyv
/// rkyv's Archive, Serialize and Deserialize have been implemented for SmallSliceBox.
/// The serialized version of a SmallSliceBox<T> is 'just' a normal `rkyv::ArchivedBox<[T]>`.
/// This is a match made in heaven, since rkyv's relative pointers use only 32 bits for the pointer part _as well as_ the length part.
/// As such, `sizeof(rkyv::Archived<SmallSliceBox<T>>) == 8` bytes (!).
/// (This is assuming rkyv's feature `size_32` is used which is the default.
/// Changing it to `size_64` is rarely useful for the same reason as the rant about lengths above.)

#[repr(packed)]
pub struct SmallSliceBox<T>
where
    T: Pointee<Metadata = usize> + ?Sized,
{
    ptr: core::ptr::NonNull<()>,
    len: u32,
    marker: PhantomData<T>,
}

#[derive(Debug)]
pub struct SliceTooLargeError{len: usize}
impl std::fmt::Display for SliceTooLargeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(f, "Pointed-to slice len {} was too large to fit in a SmallSliceBox (which can fit slices at most 2^32 elements long)", self.len)
    }
}

impl<T> SmallSliceBox<T>
where
    T: Pointee<Metadata = usize> + ?Sized,
{

    /// Creates a new SmallSliceBox from the given slice.
    ///
    /// This involves cloning the slice (which will clone all elements one by one)
    /// first into a vector, which is then turned into the SmallSliceBox.
    ///
    /// Panics if the slice's length cannot fit in a u32.
    pub fn new(slice: &T) -> Self
    where
        T: Clone,
    {
        Self::try_new(slice).unwrap()
    }

    /// Creates a new SmallSliceBox from the given slice.
    ///
    /// This involves cloning the slice (which will clone all elements one by one)
    /// first into a vector, which is then turned into the SmallSliceBox.
    ///
    /// # Safety
    /// The caller must ensure that `slice`'s length can fit in a u32.
    pub unsafe fn new_unchecked(slice: &T) -> Self
    where
        T: Clone,
    {
        Self::try_new(slice).unwrap_unchecked()
    }

    /// Variant of `new` which will return an error if the slice is too long instead of panicing.
    pub fn try_new(slice: &T) -> Result<Self, SliceTooLargeError>
    where
        T: Clone,
    {
        let layout = std::alloc::Layout::for_value(slice);
        let target_ptr = unsafe { std::alloc::alloc(layout) } as *mut T;
        // SAFETY: We write into newly allocated space
        unsafe { target_ptr.write(slice.clone()) };

        // SAFETY: We pass a newly allocated, filled, ptr
        unsafe { Self::try_from_raw(target_ptr) }
    }

    /// Variant of `from_box` which will return an error if the slice is too long instead of panicing.
    pub fn try_from_box(boxed: Box<T>) -> Result<Self, SliceTooLargeError> {
        let layout = std::alloc::Layout::for_value(&*boxed);
        let fat_ptr = Box::into_raw(boxed);
        // SAFETY: Box ensures fat_ptr is non-null
        unsafe { Self::try_from_raw(fat_ptr) }
    }

    /// Builds a new SmallSliceBox from a raw mutable pointer
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

    /// Builds a new SmallSliceBox from a raw mutable pointer
    ///

    /// Variant of `from_box` which will return an error if the slice is too long instead of panicing.
    pub unsafe fn try_from_raw(target_ptr: *mut T) -> Result<Self, SliceTooLargeError> {
        let (thin_ptr, len) = target_ptr.to_raw_parts();
        let small_len = len.try_into().map_err(|_| SliceTooLargeError{len})?;

        // SAFETY: Box ensures its ptr is never null.
        let ptr = unsafe { core::ptr::NonNull::new_unchecked(thin_ptr as *mut ()) };
        Ok(Self {
            ptr,
            len: small_len,
            marker: PhantomData,
        })
    }

    /// Turns a Box into a SmallSliceBox.
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

    /// Turns a SmallSliceBox into a box.
    ///
    /// This is a fast constant-time operation that needs no allocation.
    ///
    /// Not an associated function to not interfere with Deref
    pub fn into_box(this: Self) -> Box<T> {
        let ptr = Self::into_raw(this);
        // SAFETY: We reconstruct using the inverse operations from construction
        unsafe { Box::from_raw(ptr) }
    }

    /// Obtains a raw read-only pointer view of the contents of this SmallSliceBox.
    ///
    /// The resulting pointer is guaranteed to be a valid instance of T and non-null.
    fn to_ptr(this: &Self) -> *const T {
        let ptr = ptr_meta::from_raw_parts(this.ptr.as_ptr(), this.len as usize);
        ptr
    }

    /// Turns the SmallSliceBox into a raw pointer
    ///
    /// The resulting pointer is guaranteed to be a valid instance of T and non-null.
    ///
    /// Calling this function is safe, but most operations on the result are not.
    /// Similar caveats apply as to Box::into_raw.
    fn into_raw(this: Self) -> *mut T {
        let ptr = ptr_meta::from_raw_parts_mut(this.ptr.as_ptr(), this.len as usize);
        // Make sure the pointer remains valid; Caller is now responsible for managing the memory:
        core::mem::forget(this);
        ptr
    }
}

impl<T> Drop for SmallSliceBox<T>
where
    T: Pointee<Metadata = usize> + ?Sized,
{
    fn drop(&mut self) {
        let me = std::mem::replace(self, SmallSliceBox { ptr: NonNull::dangling(), len: 0, marker: PhantomData});
        core::mem::forget(self);
        let _drop_this_box = SmallSliceBox::into_box(me);
    }
}

impl<T> Deref for SmallSliceBox<T>
where
    T: Pointee<Metadata = usize> + ?Sized,
{
    type Target = T;
    fn deref(&self) -> &Self::Target {
        // SAFETY: Correct by construction
        let ptr = unsafe { ptr_meta::from_raw_parts(self.ptr.as_ptr(), self.len as usize) };
        unsafe { &*ptr }
    }
}

impl<T> DerefMut for SmallSliceBox<T>
    where
    T: Pointee<Metadata = usize> + ?Sized,
{
    fn deref_mut(&mut self) -> &mut Self::Target {
        // SAFETY: Correct by construction
        let ptr = unsafe { ptr_meta::from_raw_parts_mut(self.ptr.as_ptr(), self.len as usize) };
        unsafe { &mut  *ptr }
    }
}

impl<T> core::borrow::Borrow<T> for SmallSliceBox<T>
where
    T: Pointee<Metadata = usize> + ?Sized,
{
    fn borrow(&self) -> &T {
        &**self
    }
}

impl<T> core::borrow::BorrowMut<T> for SmallSliceBox<T>
where
    T: Pointee<Metadata = usize> + ?Sized,
{
    fn borrow_mut(&mut self) -> &mut T {
        &mut **self
    }
}

impl<T> AsRef<T> for SmallSliceBox<T>
where
    T: Pointee<Metadata = usize> + ?Sized,
{
    fn as_ref(&self) -> &T {
        &**self
    }
}


impl<T> AsMut<T> for SmallSliceBox<T>
where
    T: Pointee<Metadata = usize> + ?Sized,
{
    fn as_mut(&mut self) -> &mut T {
        &mut **self
    }
}

impl<T> Unpin for SmallSliceBox<T>
where
    T: Pointee<Metadata = usize> + ?Sized,
{}

impl<T> Clone for SmallSliceBox<T>
    where
    T: Clone,
    T: Pointee<Metadata = usize> + ?Sized,
{
    fn clone(&self) -> Self {
        let slice = self.deref();
        // SAFETY: The original SmallSliceBox already checked this invariant on construction
        unsafe { SmallSliceBox::new_unchecked(slice) }
    }
}

impl<T: core::fmt::Debug> core::fmt::Debug for SmallSliceBox<T>
where
    T: Pointee<Metadata = usize> + ?Sized,
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
    RemoteString{ptr: SmallSliceBox<str>},
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



// pub struct SmallSliceBoxResolver<T>(rkyv::boxed::BoxResolver<<[T] as ArchiveUnsized>::MetadataResolver>)
// where
//     Box<[T]>: Archive,
//     [T]: ArchiveUnsized;

// /// SmallSliceBox is archived into an ArchivedBox<[T]>, just like a normal box.
// impl<T> Archive for SmallSliceBox<T>
//     where
//     Box<[T]>: Archive,
//     [T]: ArchiveUnsized,
// {
//     type Archived = ArchivedBox<<[T] as ArchiveUnsized>::Archived>;
//     type Resolver = SmallSliceBoxResolver<T>;

//     #[inline]
//     unsafe fn resolve(&self, pos: usize, resolver: Self::Resolver, out: *mut Self::Archived) {
//         println!("Resolving {:?}", &self as *const _);
//         rkyv::boxed::ArchivedBox::resolve_from_ref(self.as_ref(), pos, resolver.0, out)
//     }
// }

// impl<S: rkyv::Fallible + ?Sized, T> Serialize<S> for SmallSliceBox<T>
//     where
//     Box<[T]>: Serialize<S>,
//     [T]: SerializeUnsized<S>,
// {
//     #[inline]
//     fn serialize(&self, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
//         println!("Serializing {:?}", &self as *const _);
//         let res = ArchivedBox::serialize_from_ref(self.as_ref(), serializer)?;
//         Ok(SmallSliceBoxResolver(res))
//     }
// }

// impl<T, D> Deserialize<SmallSliceBox<T>, D> for ArchivedBox<<[T] as ArchiveUnsized>::Archived>
//     where
//     [T]: ArchiveUnsized,
//     <[T] as ArchiveUnsized>::Archived: rkyv::DeserializeUnsized<[T], D>,
//     D: rkyv::Fallible + ?Sized,
// {
//     fn deserialize(&self, deserializer: &mut D) -> Result<SmallSliceBox<T>, D::Error> {
//         println!("Deserializing {:?}", &self as *const _);
//         let boxed: Box<[T]> = self.deserialize(deserializer)?;
//         Ok(SmallSliceBox::from_box(boxed))
//     }
// }


#[cfg(test)]
mod tests {
    use crate::SmallSliceBox;


    #[test]
    fn rkyv() {
        let boxed = SmallSliceBox::new(&[1,2,3,4].as_slice());
        println!("{:?}", &boxed);
        let bytes = rkyv::to_bytes::<_, 64>(&boxed).unwrap();
        println!("{:?}", bytes);
        let deserialized: SmallSliceBox<i32> = unsafe { rkyv::from_bytes_unchecked(&bytes) }.unwrap();
        println!("{:?}", deserialized);

    }
}
