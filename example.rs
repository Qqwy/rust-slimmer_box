#![feature(prelude_import)]
#![feature(rustc_attrs)]
#[prelude_import]
use std::prelude::rust_2021::*;
#[macro_use]
extern crate std;
use std::{mem::ManuallyDrop, ptr::NonNull, marker::PhantomData, ops::{Deref, DerefMut}};
use bytecheck::CheckBytes;
use rkyv::{
    Archive, Deserialize, Serialize, Archived, Resolver, ArchiveUnsized,
    boxed::ArchivedBox, SerializeUnsized,
};
use trace::trace;
const DEPTH: ::std::thread::LocalKey<::std::cell::Cell<usize>> = {
    #[inline]
    fn __init() -> ::std::cell::Cell<usize> {
        ::std::cell::Cell::new(0)
    }
    #[inline]
    unsafe fn __getit(
        init: ::std::option::Option<&mut ::std::option::Option<::std::cell::Cell<usize>>>,
    ) -> ::std::option::Option<&'static ::std::cell::Cell<usize>> {
        #[thread_local]
        #[cfg(
            all(
                target_thread_local,
                not(all(target_family = "wasm", not(target_feature = "atomics"))),
            )
        )]
        static __KEY: ::std::thread::__FastLocalKeyInner<::std::cell::Cell<usize>> = ::std::thread::__FastLocalKeyInner::new();
        #[allow(unused_unsafe)]
        unsafe {
            __KEY
                .get(move || {
                    if let ::std::option::Option::Some(init) = init {
                        if let ::std::option::Option::Some(value) = init.take() {
                            return value;
                        } else if true {
                            ::core::panicking::panic_fmt(
                                ::core::fmt::Arguments::new_v1(
                                    &["internal error: entered unreachable code: "],
                                    &[
                                        ::core::fmt::ArgumentV1::new_display(
                                            &::core::fmt::Arguments::new_v1(
                                                &["missing default value"],
                                                &[],
                                            ),
                                        ),
                                    ],
                                ),
                            );
                        }
                    }
                    __init()
                })
        }
    }
    unsafe { ::std::thread::LocalKey::new(__getit) }
};
#[repr(transparent)]
pub struct Foo<T>(SmallSliceBox<T>);
#[automatically_derived]
///An archived [`Foo`]
#[repr()]
pub struct ArchivedFoo<T>(
    ///The archived counterpart of [`Foo::0`]
    ::rkyv::Archived<SmallSliceBox<T>>,
)
where
    SmallSliceBox<T>: ::rkyv::Archive;
#[automatically_derived]
///The resolver for an archived [`Foo`]
pub struct FooResolver<T>(
    ::rkyv::Resolver<SmallSliceBox<T>>,
)
where
    SmallSliceBox<T>: ::rkyv::Archive;
#[automatically_derived]
const _: () = {
    use ::core::marker::PhantomData;
    use ::rkyv::{out_field, Archive, Archived};
    impl<T> Archive for Foo<T>
    where
        SmallSliceBox<T>: ::rkyv::Archive,
    {
        type Archived = ArchivedFoo<T>;
        type Resolver = FooResolver<T>;
        #[allow(clippy::unit_arg)]
        #[inline]
        unsafe fn resolve(
            &self,
            pos: usize,
            resolver: Self::Resolver,
            out: *mut Self::Archived,
        ) {
            let (fp, fo) = {
                #[allow(unused_unsafe)]
                unsafe {
                    let fo = &raw mut (*out).0;
                    (fo.cast::<u8>().offset_from(out.cast::<u8>()) as usize, fo)
                }
            };
            ::rkyv::Archive::resolve((&self.0), pos + fp, resolver.0, fo);
        }
    }
};
#[automatically_derived]
const _: () = {
    use ::rkyv::{Archive, Fallible, Serialize};
    impl<__S: Fallible + ?Sized, T> Serialize<__S> for Foo<T>
    where
        SmallSliceBox<T>: Serialize<__S>,
    {
        #[inline]
        fn serialize(
            &self,
            serializer: &mut __S,
        ) -> ::core::result::Result<Self::Resolver, __S::Error> {
            Ok(FooResolver(Serialize::<__S>::serialize(&self.0, serializer)?))
        }
    }
};
#[automatically_derived]
const _: () = {
    use ::rkyv::{Archive, Archived, Deserialize, Fallible};
    impl<__D: Fallible + ?Sized, T> Deserialize<Foo<T>, __D> for Archived<Foo<T>>
    where
        SmallSliceBox<T>: Archive,
        Archived<SmallSliceBox<T>>: Deserialize<SmallSliceBox<T>, __D>,
    {
        #[inline]
        fn deserialize(
            &self,
            deserializer: &mut __D,
        ) -> ::core::result::Result<Foo<T>, __D::Error> {
            Ok(
                Foo(
                    Deserialize::<
                        SmallSliceBox<T>,
                        __D,
                    >::deserialize(&self.0, deserializer)?,
                ),
            )
        }
    }
};
/// A packed alternative to Box<[T]> for slices with at most 2ˆ32 (4_294_967_296) elements.
///
/// A normal Box<[T]> is an owned 'fat pointer' that contains both the pointer to memory
/// as well as the size (as an usize) of the managed slice.
///
/// On 64-bit targets (where sizeof(usize) == sizeof(u64)), this makes a `Box<[T]>` take up 16 bytes.
///
/// But it is rather common to work with slices that will never be that large:
/// a [u8; 2^32] takes 4GiB of space. Are you really working with strings that are larger in your app?
///
/// And since the length is counted in elements, a [u62; 2^32] takes 32GiB.
///
/// By storing the length of such a 'fat pointer' inside a u32 rather than a u64,
/// a SmallSliceBox only takes up 12 bytes (96 bits, 1.5 words) rather than 16 bytes (128 bits, 2 words).
///
/// This allows it to be used inside another structure, such as in one or more variants of an enum.
/// The resulting structure will then still only take up 16 bytes (2 words).
///
/// # Rkyv
/// rkyv's Archive, Serialize and Deserialize have been implemented for SmallSliceBox.
/// The serialized version of a SmallSliceBox<T> is 'just' a normal `rkyv::ArchivedBox<[T]>`.
/// This is perfectly fine, since rkyv's relative pointers use only 32 bits for the pointer part.
/// (This is assuming rkyv's feature `size_32` is used which is the default.)
/// As such, `sizeof(rkyv::Archived<SmallSliceBox<T>>) == 12` as well.
#[repr(packed)]
pub struct SmallSliceBox<T> {
    ptr: core::ptr::NonNull<T>,
    size: u32,
    marker: core::marker::PhantomData<T>,
}
pub struct SmallSliceBoxResolver<T>(
    rkyv::boxed::BoxResolver<<[T] as ArchiveUnsized>::MetadataResolver>,
)
where
    Box<[T]>: Archive,
    [T]: ArchiveUnsized;
/// SmallSliceBox is archived into an ArchivedBox<[T]>, just like a normal box.
impl<T> Archive for SmallSliceBox<T>
where
    Box<[T]>: Archive,
    [T]: ArchiveUnsized,
{
    type Archived = ArchivedBox<<[T] as ArchiveUnsized>::Archived>;
    type Resolver = SmallSliceBoxResolver<T>;
    #[inline]
    unsafe fn resolve(
        &self,
        pos: usize,
        resolver: Self::Resolver,
        out: *mut Self::Archived,
    ) {
        {
            ::std::io::_print(
                ::core::fmt::Arguments::new_v1(
                    &["Resolving ", "\n"],
                    &[::core::fmt::ArgumentV1::new_debug(&(&self as *const _))],
                ),
            );
        };
        rkyv::boxed::ArchivedBox::resolve_from_ref(self.as_ref(), pos, resolver.0, out)
    }
}
impl<S: rkyv::Fallible + ?Sized, T> Serialize<S> for SmallSliceBox<T>
where
    Box<[T]>: Serialize<S>,
    [T]: SerializeUnsized<S>,
{
    #[inline]
    fn serialize(&self, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
        {
            ::std::io::_print(
                ::core::fmt::Arguments::new_v1(
                    &["Serializing ", "\n"],
                    &[::core::fmt::ArgumentV1::new_debug(&(&self as *const _))],
                ),
            );
        };
        let res = ArchivedBox::serialize_from_ref(self.as_ref(), serializer)?;
        Ok(SmallSliceBoxResolver(res))
    }
}
impl<T, D> Deserialize<SmallSliceBox<T>, D>
for ArchivedBox<<[T] as ArchiveUnsized>::Archived>
where
    [T]: ArchiveUnsized,
    <[T] as ArchiveUnsized>::Archived: rkyv::DeserializeUnsized<[T], D>,
    D: rkyv::Fallible + ?Sized,
{
    fn deserialize(&self, deserializer: &mut D) -> Result<SmallSliceBox<T>, D::Error> {
        {
            ::std::io::_print(
                ::core::fmt::Arguments::new_v1(
                    &["Deserializing ", "\n"],
                    &[::core::fmt::ArgumentV1::new_debug(&(&self as *const _))],
                ),
            );
        };
        let boxed: Box<[T]> = self.deserialize(deserializer)?;
        Ok(SmallSliceBox::from_box(boxed))
    }
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
    pub fn try_from_box(
        boxed: Box<[T]>,
    ) -> Result<Self, <u32 as TryFrom<usize>>::Error> {
        {
            ::std::io::_print(::core::fmt::Arguments::new_v1(&["Hello\n"], &[]));
        };
        let size = boxed.len().try_into()?;
        let fat_ptr = Box::into_raw(boxed);
        let thin_ptr = fat_ptr as *mut T;
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
}
impl<T> SmallSliceBox<T> {
    /// Turns a SmallSliceBox into a box.
    ///
    /// This is a fast constant-time operation that needs no allocation.
    ///
    /// Not an associated function to not interfere with Deref
    pub fn to_box(this: Self) -> Box<[T]> {
        {
            ::std::io::_print(::core::fmt::Arguments::new_v1(&["to_box called\n"], &[]));
        };
        let ptr = core::ptr::slice_from_raw_parts(this.ptr.as_ptr(), this.size as usize)
            as *mut _;
        let res = unsafe { Box::from_raw(ptr) };
        core::mem::forget(this);
        res
    }
}
impl<T> Drop for SmallSliceBox<T> {
    fn drop(&mut self) {
        let me = std::mem::replace(
            self,
            SmallSliceBox {
                ptr: NonNull::dangling(),
                size: 0,
                marker: PhantomData,
            },
        );
        core::mem::forget(self);
        let _drop_this_box = SmallSliceBox::to_box(me);
    }
}
impl<T> Deref for SmallSliceBox<T> {
    type Target = [T];
    fn deref(&self) -> &Self::Target {
        unsafe { core::slice::from_raw_parts(self.ptr.as_ptr(), self.size as usize) }
    }
}
impl<T> DerefMut for SmallSliceBox<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
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
    T: Clone,
{
    fn clone(&self) -> Self {
        let slice = self.deref();
        unsafe { SmallSliceBox::new_unchecked(slice) }
    }
}
impl<T: core::fmt::Debug> core::fmt::Debug for SmallSliceBox<T> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        core::fmt::Debug::fmt(&**self, f)
    }
}
pub enum Thing {
    LocalString { bytes: [u8; 14], len: u8 },
    RemoteString { ptr: SmallSliceBox<u8> },
}
