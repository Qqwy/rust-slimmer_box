#![feature(rustc_attrs)]
use std::{mem::ManuallyDrop, ptr::NonNull, marker::PhantomData, ops::{Deref, DerefMut}};

use bytecheck::CheckBytes;
use rkyv::{Archive, Deserialize, Serialize};

// #[rustc_layout(debug)]
#[repr(packed)]
pub struct SmallSliceBox<T> {
    ptr: core::ptr::NonNull<T>,
    size: u32,
    marker: core::marker::PhantomData<T>,
}

impl<T> SmallSliceBox<T> {

    /// Creates a new SmallSliceBox from the given slice.
    ///
    /// This involves cloning the slice (which will clone all elements one by one)
    /// first into a vector, which is then turned into the SmallSliceBox.
    ///
    /// Panics if the slice's length cannot fit in a u32.
    pub fn new(slice: &[T]) -> Self
    where
        T: Clone,
    {
        let boxed: Box<[T]> = slice.to_vec().into_boxed_slice();
        Self::from_box(boxed)
    }

    /// Creates a new SmallSliceBox from the given slice.
    ///
    /// This involves cloning the slice (which will clone all elements one by one)
    /// first into a vector, which is then turned into the SmallSliceBox.
    ///
    /// # Safety
    /// The caller must ensure that `slice`'s length can fit in a u32.
    pub unsafe fn new_unchecked(slice: &[T]) -> Self
    where
        T: Clone,
    {
        let boxed: Box<[T]> = slice.to_vec().into_boxed_slice();
        Self::from_box_unchecked(boxed)
    }

    /// Variant of `new` which will return an error if the slice is too long instead of panicing.
    pub fn try_new(slice: &[T]) -> Result<Self, <u32 as TryFrom<usize>>::Error>
    where
        T: Clone,
    {
        let boxed: Box<[T]> = slice.to_vec().into_boxed_slice();
        Self::try_from_box(boxed)
    }

    /// Optimization of `new` for types implementing `Copy`
    ///
    /// Does not need to work with an intermediate vec,
    /// and creating a copy from the slice is much faster.
    ///
    /// Panics if the slice's length cannot fit in a u32.
    pub fn new_from_copy(slice: &[T]) -> Self
    where
        T: Copy,
    {
        let boxed: Box<[T]> = slice.into();
        Self::from_box(boxed)
    }

    /// Variant of `new_from_copy` which will return an error if the slice is too long instead of panicing.
    pub fn try_new_from_copy(slice: &[T]) -> Result<Self, <u32 as TryFrom<usize>>::Error>
    where
        T: Copy,
    {
        let boxed: Box<[T]> = slice.into();
        Self::try_from_box(boxed)
    }

    /// Variant of `from_box` which will return an error if the slice is too long instead of panicing.
    pub fn try_from_box(boxed: Box<[T]>) -> Result<Self, <u32 as TryFrom<usize>>::Error> {
        let size = boxed.len().try_into()?;
        let fat_ptr = Box::into_raw(boxed);
        let thin_ptr = fat_ptr as *mut T; // NOTE: Is there a nicer way to do this?
        // SAFETY: Box ensures its ptr is never null.
        let ptr = unsafe { core::ptr::NonNull::new_unchecked(thin_ptr) };
        let res = SmallSliceBox {
            ptr,
            size,
            marker: core::marker::PhantomData,
        };
        Ok(res)
    }

    /// Turns a Box into a SmallSliceBox.
    ///
    /// This is a fast constant-time operation that needs no allocation.
    ///
    /// Panics if the slice's length cannot fit in a u32.
    pub fn from_box(boxed: Box<[T]>) -> Self {
        Self::try_from_box(boxed).unwrap()
    }

    /// Variant of `from_box` that will not check whether the slice is short enough.
    ///
    /// # Safety
    /// - The caller needs to ensure that the slice never has more elements than can fit in a u32.
    pub unsafe fn from_box_unchecked(boxed: Box<[T]>) -> Self {
        Self::try_from_box(boxed).unwrap_unchecked()
    }

    /// Turns a SmallSliceBox into a box.
    ///
    /// This is a fast constant-time operation that needs no allocation.
    ///
    /// Not an associated function to not interfere with Deref
    pub fn to_box(this: Self) -> Box<[T]> {
        // SAFETY: We reconstruct using the inverse operations from before
        let ptr = core::ptr::slice_from_raw_parts(this.ptr.as_ptr(), this.size as usize) as *mut _;
        unsafe { Box::from_raw(ptr) }
    }
}

impl<T> Drop for SmallSliceBox<T> {
    fn drop(&mut self) {
        let me = std::mem::replace(self, SmallSliceBox { ptr: NonNull::dangling(), size: 0, marker: PhantomData});
        core::mem::forget(self);
        let _drop_this_box = SmallSliceBox::to_box(me);
    }
}

impl<T> Deref for SmallSliceBox<T> {
    type Target = [T];
    fn deref(&self) -> &Self::Target {
        // SAFETY: Correct by construction
        unsafe { core::slice::from_raw_parts(self.ptr.as_ptr(), self.size as usize) }
    }
}

impl<T> DerefMut for SmallSliceBox<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        // SAFETY: Correct by construction
        unsafe { core::slice::from_raw_parts_mut(self.ptr.as_ptr(), self.size as usize) }
    }
}

impl<T> core::borrow::Borrow<[T]> for SmallSliceBox<T> {
    fn borrow(&self) -> &[T] {
        &**self
    }
}

impl<T> core::borrow::BorrowMut<[T]> for SmallSliceBox<T> {
    fn borrow_mut(&mut self) -> &mut [T] {
        &mut **self
    }
}

impl<T> AsRef<[T]> for SmallSliceBox<T> {
    fn as_ref(&self) -> &[T] {
        &**self
    }
}


impl<T> AsMut<[T]> for SmallSliceBox<T> {
    fn as_mut(&mut self) -> &mut [T] {
        &mut **self
    }
}

impl<T> Unpin for SmallSliceBox<T> {}

impl<T> Clone for SmallSliceBox<T>
    where
    T: Clone
{
    fn clone(&self) -> Self {
        let slice = self.deref();
        // SAFETY: The original SmallSliceBox already checked this invariant on construction
        unsafe { SmallSliceBox::new_unchecked(slice) }
    }
}

impl<T: core::fmt::Debug> core::fmt::Debug for SmallSliceBox<T> {
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
    RemoteString{ptr: SmallSliceBox<u8>},
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
