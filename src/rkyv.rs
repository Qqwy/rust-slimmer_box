use alloc::boxed::Box;
use ptr_meta::Pointee;

use crate::SlimmerBox;
use crate::slim_pointee::SlimmerPointee;

use rkyv::{Archive, Deserialize, Serialize, ArchiveUnsized, boxed::ArchivedBox, SerializeUnsized};


pub struct SlimmerBoxResolver<T>(rkyv::boxed::BoxResolver<<T as ArchiveUnsized>::MetadataResolver>)
where
    T: ?Sized,

    Box<T>: Archive,
    T: ArchiveUnsized;

/// SlimmerBox is archived into an ArchivedBox<T>, just like a normal box.
impl<T, SlimmerMetadata> Archive for SlimmerBox<T, SlimmerMetadata>
    where
    T: ?Sized,
    T: SlimmerPointee<SlimmerMetadata>,
    SlimmerMetadata: TryFrom<<T as Pointee>::Metadata> + TryInto<<T as Pointee>::Metadata> + Copy,

    Box<T>: Archive,
    T: ArchiveUnsized,
{
    type Archived = ArchivedBox<<T as ArchiveUnsized>::Archived>;
    type Resolver = SlimmerBoxResolver<T>;

    #[inline]
    unsafe fn resolve(&self, pos: usize, resolver: Self::Resolver, out: *mut Self::Archived) {
        rkyv::boxed::ArchivedBox::resolve_from_ref(self.as_ref(), pos, resolver.0, out)
    }
}

impl<S: rkyv::Fallible + ?Sized, T, SlimmerMetadata> Serialize<S> for SlimmerBox<T, SlimmerMetadata>
    where
    T: ?Sized,
    T: SlimmerPointee<SlimmerMetadata>,
    SlimmerMetadata: TryFrom<<T as Pointee>::Metadata> + TryInto<<T as Pointee>::Metadata> + Copy,

    Box<T>: Serialize<S>,
    T: SerializeUnsized<S>,
{
    #[inline]
    fn serialize(&self, serializer: &mut S) -> Result<Self::Resolver, S::Error> {
        let res = ArchivedBox::serialize_from_ref(self.as_ref(), serializer)?;
        Ok(SlimmerBoxResolver(res))
    }
}

impl<T, D, SlimmerMetadata> Deserialize<SlimmerBox<T, SlimmerMetadata>, D> for ArchivedBox<<T as ArchiveUnsized>::Archived>
    where
    T: ?Sized,
    T: SlimmerPointee<SlimmerMetadata>,
    SlimmerMetadata: TryFrom<<T as Pointee>::Metadata> + TryInto<<T as Pointee>::Metadata> + Copy,

    T: ArchiveUnsized,
    <T as ArchiveUnsized>::Archived: rkyv::DeserializeUnsized<T, D>,
    D: rkyv::Fallible + ?Sized,
{
    fn deserialize(&self, deserializer: &mut D) -> Result<SlimmerBox<T, SlimmerMetadata>, D::Error> {
        let boxed: Box<T> = self.deserialize(deserializer)?;
        Ok(SlimmerBox::from_box(boxed))
    }
}


#[cfg(test)]
mod tests {
    use crate::SlimmerBox;

    #[test]
    fn rkyv_roundtrip() {
        let boxed: SlimmerBox<[i32]> = SlimmerBox::new([1,2,3,4].as_slice());
        let bytes = rkyv::to_bytes::<_, 64>(&boxed).unwrap();
        assert_eq!(&bytes[..], &[1, 0, 0, 0, 2, 0, 0, 0, 3, 0, 0, 0, 4, 0, 0, 0, 240, 255, 255, 255, 4, 0, 0, 0][..]);
        let deserialized: SlimmerBox<[i32], u32> = unsafe { rkyv::from_bytes_unchecked(&bytes) }.unwrap();
        assert_eq!(*boxed, *deserialized);
    }
}
