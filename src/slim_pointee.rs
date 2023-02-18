use ptr_meta::Pointee;

/// Trait which can be implemented by any pointer-like ('Pointee')types,
/// as long as their metadata might be made smaller.
///
/// As such, it is implemented for:
/// - Zero-sized types (no metadata and actually also no pointer)
/// - Statically-sized types (no metadata)
/// - Dynamically-sized types (metadata indicates length which can be made smaller if the length fits in the smaller int)
///
/// It is _not_ implemented for trait objects, because their metadata is itself a pointer!
///
/// # Safety
///
/// In a perfect world, we would have hade a `SlimmerMetadata: Into<Self as Pointee>::Metadata` bound on this trait.
/// However, since pointer sizes differ on different compilation targets,
/// current Rust never implements this conversion.
///
/// As such, this trait functions under the assumption that
/// ```ignore
/// let slim_meta = meta.try_into();
/// let recovered_meta = slim_meta.try_into().unwrap();
/// ```
/// will never fail.
///
/// This assumption is very reasonable and in all likelyhood fulfilled by any pointer metadata type
/// you ever throw at it.
/// But since we cannot trust arbitrary safe code to do 'the right thing', this trait needs to be unsafe.
pub unsafe trait SlimmerPointee<SlimmerMetadata>: Pointee
    where
    <Self as Pointee>::Metadata: Clone,
    SlimmerMetadata: TryFrom<<Self as Pointee>::Metadata> + TryInto<<Self as Pointee>::Metadata>
{
}

/// Trivial implementation for signed types, as they do not have any metadata.
unsafe impl<T: Sized> SlimmerPointee<()> for T {}

/// Implementation that will behave identical to Box<[T]> on any architecture
unsafe impl<T> SlimmerPointee<usize> for [T] {}

/// Store at most 15 elements
unsafe impl<T> SlimmerPointee<u8> for [T] {}

/// Store at most 65535 elements
unsafe impl<T> SlimmerPointee<u16> for [T] {}

/// Store at most 4294967295 elements
unsafe impl<T> SlimmerPointee<u32> for [T] {}

/// Store at most 18446744073709551615 elements
unsafe impl<T> SlimmerPointee<u64> for [T] {}

/// Implementation that will behave identical to Box<[T]> on any architecture
unsafe impl SlimmerPointee<usize> for str {}

/// Store at most 15 bytes
unsafe impl SlimmerPointee<u8> for str {}

/// Store at most 65535 bytes == 64KiB
unsafe impl SlimmerPointee<u16> for str {}

/// Store at most 4294967295 bytes == 4GiB
unsafe impl SlimmerPointee<u32> for str {}

/// Store at most 18446744073709551615 bytes == 16 EiB
unsafe impl SlimmerPointee<u64> for str {}


