use alloc::boxed::Box;
use alloc::vec::Vec;

use ptr_meta::Pointee;
use ::serde::ser::Serialize;
use ::serde::de::{Deserialize, Deserializer};

use crate::{SlimmerBox, SlimmerPointee};

impl<'de, T, SlimmerMetadata,> Deserialize<'de> for SlimmerBox<T, SlimmerMetadata>
where
    T: Sized,
    T: SlimmerPointee<SlimmerMetadata>,
    SlimmerMetadata: TryFrom<<T as Pointee>::Metadata> + TryInto<<T as Pointee>::Metadata> + Copy,
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Deserialize::deserialize(deserializer).map(|val| SlimmerBox::new(&val))
    }
}

impl<'de, T, SlimmerMetadata,> Deserialize<'de> for SlimmerBox<[T], SlimmerMetadata>
where
    T: Sized,
[T]: SlimmerPointee<SlimmerMetadata>,
    SlimmerMetadata: TryFrom<<[T] as Pointee>::Metadata> + TryInto<<[T] as Pointee>::Metadata> + Copy,
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Deserialize::deserialize(deserializer).map(|val| SlimmerBox::from_box(Vec::into_boxed_slice(val)))
    }
}

impl<'de, SlimmerMetadata,> Deserialize<'de> for SlimmerBox<str, SlimmerMetadata>
where
    str: SlimmerPointee<SlimmerMetadata>,
    SlimmerMetadata: TryFrom<<str as Pointee>::Metadata> + TryInto<<str as Pointee>::Metadata> + Copy,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Deserialize::deserialize(deserializer).map(|val| SlimmerBox::from_box(Vec::into_boxed_slice(val)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_test::{Token, assert_tokens};

    #[test]
    fn serde_round_trip() {
        let boxed: SlimmerBox<[i32]> = SlimmerBox::new([1, 2, 3, 4].as_slice());
        assert_tokens(&boxed, []);
    }
}
