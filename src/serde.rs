
use alloc::string::String;
use alloc::vec::Vec;

use ::serde::de::{Deserialize, Deserializer};
use ::serde::ser::{Serialize, Serializer};
use ptr_meta::Pointee;

use crate::{SlimmerBox, SlimmerPointee};

impl<T: Serialize, SlimmerMetadata> Serialize for SlimmerBox<T, SlimmerMetadata>
where
    T: ?Sized,
    T: SlimmerPointee<SlimmerMetadata>,
    SlimmerMetadata: TryFrom<<T as Pointee>::Metadata> + TryInto<<T as Pointee>::Metadata> + Copy,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        (**self).serialize(serializer)
    }
}

impl<'de, T: Sized + Clone, SlimmerMetadata> Deserialize<'de> for SlimmerBox<T, SlimmerMetadata>
where
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

impl<'de, T, SlimmerMetadata> Deserialize<'de> for SlimmerBox<[T], SlimmerMetadata>
where
    T: Sized,
    [T]: SlimmerPointee<SlimmerMetadata>,
    SlimmerMetadata:
        TryFrom<<[T] as Pointee>::Metadata> + TryInto<<[T] as Pointee>::Metadata> + Copy,
    T: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Deserialize::deserialize(deserializer)
            .map(|val| SlimmerBox::from_box(Vec::into_boxed_slice(val)))
    }
}

impl<'de, SlimmerMetadata> Deserialize<'de> for SlimmerBox<str, SlimmerMetadata>
where
    str: SlimmerPointee<SlimmerMetadata>,
    SlimmerMetadata:
        TryFrom<<str as Pointee>::Metadata> + TryInto<<str as Pointee>::Metadata> + Copy,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        Deserialize::deserialize(deserializer)
            .map(|val| SlimmerBox::from_box(String::into_boxed_str(val)))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_test::{assert_tokens, Token};

    #[test]
    fn serde_round_trip_sized() {
        let boxed: SlimmerBox<u64, _> = SlimmerBox::new(&64);
        assert_tokens(&boxed, &[Token::U64(64)]);
    }

    #[test]
    fn serde_round_trip_slice() {
        let boxed: SlimmerBox<[i32]> = SlimmerBox::new([1, 2, 3, 4].as_slice());
        assert_tokens(
            &boxed,
            &[
                Token::Seq { len: Some(4) },
                Token::I32(1),
                Token::I32(2),
                Token::I32(3),
                Token::I32(4),
                Token::SeqEnd,
            ],
        );
    }

    #[test]
    fn serde_round_trip_str() {
        let boxed: SlimmerBox<str> = SlimmerBox::new("hello");
        assert_tokens(&boxed, &[Token::Str("hello")]);
    }
}
