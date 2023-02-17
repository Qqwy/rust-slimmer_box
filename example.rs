#![feature(prelude_import)]
#[prelude_import]
use std::prelude::rust_2021::*;
#[macro_use]
extern crate std;
use bytecheck::CheckBytes;
use rkyv::{Archive, Deserialize, Serialize};
#[archive_attr(derive(CheckBytes))]
#[archive_attr(derive(Debug))]
#[archive(compare(PartialEq, PartialOrd))]
pub enum Thing {
    LocalString { bytes: [u8; 14], len: u8 },
    RemoteString { ptr: Box<str> },
}
#[automatically_derived]
impl ::core::fmt::Debug for Thing {
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        match self {
            Thing::LocalString { bytes: __self_0, len: __self_1 } => {
                ::core::fmt::Formatter::debug_struct_field2_finish(
                    f,
                    "LocalString",
                    "bytes",
                    &__self_0,
                    "len",
                    &__self_1,
                )
            }
            Thing::RemoteString { ptr: __self_0 } => {
                ::core::fmt::Formatter::debug_struct_field1_finish(
                    f,
                    "RemoteString",
                    "ptr",
                    &__self_0,
                )
            }
        }
    }
}
#[automatically_derived]
impl ::core::clone::Clone for Thing {
    #[inline]
    fn clone(&self) -> Thing {
        match self {
            Thing::LocalString { bytes: __self_0, len: __self_1 } => {
                Thing::LocalString {
                    bytes: ::core::clone::Clone::clone(__self_0),
                    len: ::core::clone::Clone::clone(__self_1),
                }
            }
            Thing::RemoteString { ptr: __self_0 } => {
                Thing::RemoteString {
                    ptr: ::core::clone::Clone::clone(__self_0),
                }
            }
        }
    }
}
#[automatically_derived]
///An archived [`Thing`]
#[repr(u8)]
pub enum ArchivedThing
where
    [u8; 14]: ::rkyv::Archive,
    u8: ::rkyv::Archive,
    Box<str>: ::rkyv::Archive,
{
    ///The archived counterpart of [`Thing::LocalString`]
    #[allow(dead_code)]
    LocalString {
        ///The archived counterpart of [`Thing::LocalString::bytes`]
        bytes: ::rkyv::Archived<[u8; 14]>,
        ///The archived counterpart of [`Thing::LocalString::len`]
        len: ::rkyv::Archived<u8>,
    },
    ///The archived counterpart of [`Thing::RemoteString`]
    #[allow(dead_code)]
    RemoteString {
        ///The archived counterpart of [`Thing::RemoteString::ptr`]
        ptr: ::rkyv::Archived<Box<str>>,
    },
}
#[automatically_derived]
impl ::core::fmt::Debug for ArchivedThing
where
    [u8; 14]: ::rkyv::Archive,
    u8: ::rkyv::Archive,
    Box<str>: ::rkyv::Archive,
{
    fn fmt(&self, f: &mut ::core::fmt::Formatter) -> ::core::fmt::Result {
        match self {
            ArchivedThing::LocalString { bytes: __self_0, len: __self_1 } => {
                ::core::fmt::Formatter::debug_struct_field2_finish(
                    f,
                    "LocalString",
                    "bytes",
                    &__self_0,
                    "len",
                    &__self_1,
                )
            }
            ArchivedThing::RemoteString { ptr: __self_0 } => {
                ::core::fmt::Formatter::debug_struct_field1_finish(
                    f,
                    "RemoteString",
                    "ptr",
                    &__self_0,
                )
            }
        }
    }
}
const _: () = {
    use ::core::{convert::Infallible, marker::PhantomData};
    use bytecheck::{
        CheckBytes, EnumCheckError, ErrorBox, StructCheckError, TupleStructCheckError,
    };
    #[repr(u8)]
    enum Tag {
        LocalString,
        RemoteString,
    }
    struct Discriminant;
    impl Discriminant {
        #[allow(non_upper_case_globals)]
        const LocalString: u8 = Tag::LocalString as u8;
        #[allow(non_upper_case_globals)]
        const RemoteString: u8 = Tag::RemoteString as u8;
    }
    #[repr(C)]
    struct VariantLocalString
    where
        [u8; 14]: ::rkyv::Archive,
        u8: ::rkyv::Archive,
        Box<str>: ::rkyv::Archive,
    {
        __tag: Tag,
        bytes: ::rkyv::Archived<[u8; 14]>,
        len: ::rkyv::Archived<u8>,
        __phantom: PhantomData<ArchivedThing>,
    }
    #[repr(C)]
    struct VariantRemoteString
    where
        [u8; 14]: ::rkyv::Archive,
        u8: ::rkyv::Archive,
        Box<str>: ::rkyv::Archive,
    {
        __tag: Tag,
        ptr: ::rkyv::Archived<Box<str>>,
        __phantom: PhantomData<ArchivedThing>,
    }
    impl<__C: ?Sized> CheckBytes<__C> for ArchivedThing
    where
        [u8; 14]: ::rkyv::Archive,
        u8: ::rkyv::Archive,
        Box<str>: ::rkyv::Archive,
        ::rkyv::Archived<[u8; 14]>: CheckBytes<__C>,
        ::rkyv::Archived<u8>: CheckBytes<__C>,
        ::rkyv::Archived<Box<str>>: CheckBytes<__C>,
    {
        type Error = EnumCheckError<u8>;
        unsafe fn check_bytes<'__bytecheck>(
            value: *const Self,
            context: &mut __C,
        ) -> ::core::result::Result<&'__bytecheck Self, EnumCheckError<u8>> {
            let tag = *value.cast::<u8>();
            match tag {
                Discriminant::LocalString => {
                    let value = value.cast::<VariantLocalString>();
                    <::rkyv::Archived<
                        [u8; 14],
                    > as CheckBytes<
                        __C,
                    >>::check_bytes(&raw const (*value).bytes, context)
                        .map_err(|e| EnumCheckError::InvalidStruct {
                            variant_name: "LocalString",
                            inner: StructCheckError {
                                field_name: "bytes",
                                inner: ErrorBox::new(e),
                            },
                        })?;
                    <::rkyv::Archived<
                        u8,
                    > as CheckBytes<__C>>::check_bytes(&raw const (*value).len, context)
                        .map_err(|e| EnumCheckError::InvalidStruct {
                            variant_name: "LocalString",
                            inner: StructCheckError {
                                field_name: "len",
                                inner: ErrorBox::new(e),
                            },
                        })?;
                }
                Discriminant::RemoteString => {
                    let value = value.cast::<VariantRemoteString>();
                    <::rkyv::Archived<
                        Box<str>,
                    > as CheckBytes<__C>>::check_bytes(&raw const (*value).ptr, context)
                        .map_err(|e| EnumCheckError::InvalidStruct {
                            variant_name: "RemoteString",
                            inner: StructCheckError {
                                field_name: "ptr",
                                inner: ErrorBox::new(e),
                            },
                        })?;
                }
                _ => return Err(EnumCheckError::InvalidTag(tag)),
            }
            Ok(&*value)
        }
    }
};
#[automatically_derived]
///The resolver for an archived [`Thing`]
pub enum ThingResolver
where
    [u8; 14]: ::rkyv::Archive,
    u8: ::rkyv::Archive,
    Box<str>: ::rkyv::Archive,
{
    ///The resolver for [`Thing::LocalString`]
    #[allow(dead_code)]
    LocalString {
        ///The resolver for [`Thing::LocalString::bytes`]
        bytes: ::rkyv::Resolver<[u8; 14]>,
        ///The resolver for [`Thing::LocalString::len`]
        len: ::rkyv::Resolver<u8>,
    },
    ///The resolver for [`Thing::RemoteString`]
    #[allow(dead_code)]
    RemoteString {
        ///The resolver for [`Thing::RemoteString::ptr`]
        ptr: ::rkyv::Resolver<Box<str>>,
    },
}
#[automatically_derived]
const _: () = {
    use ::core::marker::PhantomData;
    use ::rkyv::{out_field, Archive, Archived};
    #[repr(u8)]
    enum ArchivedTag {
        LocalString,
        RemoteString,
    }
    #[repr(C)]
    struct ArchivedVariantLocalString
    where
        [u8; 14]: ::rkyv::Archive,
        u8: ::rkyv::Archive,
        Box<str>: ::rkyv::Archive,
    {
        __tag: ArchivedTag,
        bytes: Archived<[u8; 14]>,
        len: Archived<u8>,
        __phantom: PhantomData<Thing>,
    }
    #[repr(C)]
    struct ArchivedVariantRemoteString
    where
        [u8; 14]: ::rkyv::Archive,
        u8: ::rkyv::Archive,
        Box<str>: ::rkyv::Archive,
    {
        __tag: ArchivedTag,
        ptr: Archived<Box<str>>,
        __phantom: PhantomData<Thing>,
    }
    impl Archive for Thing
    where
        [u8; 14]: ::rkyv::Archive,
        u8: ::rkyv::Archive,
        Box<str>: ::rkyv::Archive,
    {
        type Archived = ArchivedThing;
        type Resolver = ThingResolver;
        #[allow(clippy::unit_arg)]
        #[inline]
        unsafe fn resolve(
            &self,
            pos: usize,
            resolver: Self::Resolver,
            out: *mut Self::Archived,
        ) {
            match resolver {
                ThingResolver::LocalString {
                    bytes: resolver_bytes,
                    len: resolver_len,
                } => {
                    match self {
                        Thing::LocalString { bytes: self_bytes, len: self_len } => {
                            let out = out.cast::<ArchivedVariantLocalString>();
                            (&raw mut (*out).__tag).write(ArchivedTag::LocalString);
                            let (fp, fo) = {
                                #[allow(unused_unsafe)]
                                unsafe {
                                    let fo = &raw mut (*out).bytes;
                                    (fo.cast::<u8>().offset_from(out.cast::<u8>()) as usize, fo)
                                }
                            };
                            ::rkyv::Archive::resolve(
                                self_bytes,
                                pos + fp,
                                resolver_bytes,
                                fo,
                            );
                            let (fp, fo) = {
                                #[allow(unused_unsafe)]
                                unsafe {
                                    let fo = &raw mut (*out).len;
                                    (fo.cast::<u8>().offset_from(out.cast::<u8>()) as usize, fo)
                                }
                            };
                            ::rkyv::Archive::resolve(
                                self_len,
                                pos + fp,
                                resolver_len,
                                fo,
                            );
                        }
                        #[allow(unreachable_patterns)]
                        _ => ::core::hint::unreachable_unchecked(),
                    }
                }
                ThingResolver::RemoteString { ptr: resolver_ptr } => {
                    match self {
                        Thing::RemoteString { ptr: self_ptr } => {
                            let out = out.cast::<ArchivedVariantRemoteString>();
                            (&raw mut (*out).__tag).write(ArchivedTag::RemoteString);
                            let (fp, fo) = {
                                #[allow(unused_unsafe)]
                                unsafe {
                                    let fo = &raw mut (*out).ptr;
                                    (fo.cast::<u8>().offset_from(out.cast::<u8>()) as usize, fo)
                                }
                            };
                            ::rkyv::Archive::resolve(
                                self_ptr,
                                pos + fp,
                                resolver_ptr,
                                fo,
                            );
                        }
                        #[allow(unreachable_patterns)]
                        _ => ::core::hint::unreachable_unchecked(),
                    }
                }
            }
        }
    }
    impl PartialEq<ArchivedThing> for Thing
    where
        [u8; 14]: ::rkyv::Archive,
        u8: ::rkyv::Archive,
        Box<str>: ::rkyv::Archive,
        Archived<[u8; 14]>: PartialEq<[u8; 14]>,
        Archived<u8>: PartialEq<u8>,
        Archived<Box<str>>: PartialEq<Box<str>>,
    {
        #[inline]
        fn eq(&self, other: &ArchivedThing) -> bool {
            match self {
                Thing::LocalString { bytes: self_bytes, len: self_len } => {
                    match other {
                        ArchivedThing::LocalString {
                            bytes: other_bytes,
                            len: other_len,
                        } => true && other_bytes.eq(self_bytes) && other_len.eq(self_len),
                        #[allow(unreachable_patterns)]
                        _ => false,
                    }
                }
                Thing::RemoteString { ptr: self_ptr } => {
                    match other {
                        ArchivedThing::RemoteString { ptr: other_ptr } => {
                            true && other_ptr.eq(self_ptr)
                        }
                        #[allow(unreachable_patterns)]
                        _ => false,
                    }
                }
            }
        }
    }
    impl PartialEq<Thing> for ArchivedThing
    where
        [u8; 14]: ::rkyv::Archive,
        u8: ::rkyv::Archive,
        Box<str>: ::rkyv::Archive,
        Archived<[u8; 14]>: PartialEq<[u8; 14]>,
        Archived<u8>: PartialEq<u8>,
        Archived<Box<str>>: PartialEq<Box<str>>,
    {
        #[inline]
        fn eq(&self, other: &Thing) -> bool {
            other.eq(self)
        }
    }
    impl PartialOrd<ArchivedThing> for Thing
    where
        [u8; 14]: ::rkyv::Archive,
        u8: ::rkyv::Archive,
        Box<str>: ::rkyv::Archive,
        Archived<[u8; 14]>: PartialOrd<[u8; 14]>,
        Archived<u8>: PartialOrd<u8>,
        Archived<Box<str>>: PartialOrd<Box<str>>,
    {
        #[inline]
        fn partial_cmp(&self, other: &ArchivedThing) -> Option<::core::cmp::Ordering> {
            let self_disc = match self {
                Thing::LocalString { .. } => 0usize,
                Thing::RemoteString { .. } => 1usize,
            };
            let other_disc = match other {
                ArchivedThing::LocalString { .. } => 0usize,
                ArchivedThing::RemoteString { .. } => 1usize,
            };
            if self_disc == other_disc {
                match self {
                    Thing::LocalString { bytes: self_bytes, len: self_len } => {
                        match other {
                            ArchivedThing::LocalString {
                                bytes: other_bytes,
                                len: other_len,
                            } => {
                                match other_bytes.partial_cmp(self_bytes) {
                                    Some(::core::cmp::Ordering::Equal) => {}
                                    cmp => return cmp,
                                }
                                match other_len.partial_cmp(self_len) {
                                    Some(::core::cmp::Ordering::Equal) => {}
                                    cmp => return cmp,
                                }
                                Some(::core::cmp::Ordering::Equal)
                            }
                            #[allow(unreachable_patterns)]
                            _ => unsafe { ::core::hint::unreachable_unchecked() }
                        }
                    }
                    Thing::RemoteString { ptr: self_ptr } => {
                        match other {
                            ArchivedThing::RemoteString { ptr: other_ptr } => {
                                match other_ptr.partial_cmp(self_ptr) {
                                    Some(::core::cmp::Ordering::Equal) => {}
                                    cmp => return cmp,
                                }
                                Some(::core::cmp::Ordering::Equal)
                            }
                            #[allow(unreachable_patterns)]
                            _ => unsafe { ::core::hint::unreachable_unchecked() }
                        }
                    }
                }
            } else {
                self_disc.partial_cmp(&other_disc)
            }
        }
    }
    impl PartialOrd<Thing> for ArchivedThing
    where
        [u8; 14]: ::rkyv::Archive,
        u8: ::rkyv::Archive,
        Box<str>: ::rkyv::Archive,
        Archived<[u8; 14]>: PartialOrd<[u8; 14]>,
        Archived<u8>: PartialOrd<u8>,
        Archived<Box<str>>: PartialOrd<Box<str>>,
    {
        #[inline]
        fn partial_cmp(&self, other: &Thing) -> Option<::core::cmp::Ordering> {
            match other.partial_cmp(self) {
                Some(::core::cmp::Ordering::Less) => Some(::core::cmp::Ordering::Greater),
                Some(::core::cmp::Ordering::Greater) => Some(::core::cmp::Ordering::Less),
                cmp => cmp,
            }
        }
    }
};
#[automatically_derived]
const _: () = {
    use ::rkyv::{Archive, Fallible, Serialize};
    impl<__S: Fallible + ?Sized> Serialize<__S> for Thing
    where
        [u8; 14]: Serialize<__S>,
        u8: Serialize<__S>,
        Box<str>: Serialize<__S>,
    {
        #[inline]
        fn serialize(
            &self,
            serializer: &mut __S,
        ) -> ::core::result::Result<Self::Resolver, __S::Error> {
            Ok(
                match self {
                    Self::LocalString { bytes, len } => {
                        ThingResolver::LocalString {
                            bytes: Serialize::<__S>::serialize(bytes, serializer)?,
                            len: Serialize::<__S>::serialize(len, serializer)?,
                        }
                    }
                    Self::RemoteString { ptr } => {
                        ThingResolver::RemoteString {
                            ptr: Serialize::<__S>::serialize(ptr, serializer)?,
                        }
                    }
                },
            )
        }
    }
};
#[automatically_derived]
const _: () = {
    use ::rkyv::{Archive, Archived, Deserialize, Fallible};
    impl<__D: Fallible + ?Sized> Deserialize<Thing, __D> for Archived<Thing>
    where
        [u8; 14]: Archive,
        Archived<[u8; 14]>: Deserialize<[u8; 14], __D>,
        u8: Archive,
        Archived<u8>: Deserialize<u8, __D>,
        Box<str>: Archive,
        Archived<Box<str>>: Deserialize<Box<str>, __D>,
    {
        #[inline]
        fn deserialize(
            &self,
            deserializer: &mut __D,
        ) -> ::core::result::Result<Thing, __D::Error> {
            Ok(
                match self {
                    Self::LocalString { bytes, len } => {
                        Thing::LocalString {
                            bytes: Deserialize::<
                                [u8; 14],
                                __D,
                            >::deserialize(bytes, deserializer)?,
                            len: Deserialize::<u8, __D>::deserialize(len, deserializer)?,
                        }
                    }
                    Self::RemoteString { ptr } => {
                        Thing::RemoteString {
                            ptr: Deserialize::<
                                Box<str>,
                                __D,
                            >::deserialize(ptr, deserializer)?,
                        }
                    }
                },
            )
        }
    }
};
