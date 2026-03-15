use crate::hashing::Hash;
use blake3::{Hash as Blake3Hash, OUT_LEN};
use redb::{Key, TypeName, Value};
use std::cmp::Ordering;

impl Value for Hash {
    type SelfType<'a> = Hash;
    type AsBytes<'a> = [u8; OUT_LEN];

    fn fixed_width() -> Option<usize> {
        Some(OUT_LEN)
    }

    fn from_bytes<'a>(data: &'a [u8]) -> Self::SelfType<'a>
    where
        Self: 'a,
    {
        let hash = Blake3Hash::from_slice(data).expect("Failed to deserialize hash");
        Hash::new(hash)
    }

    fn as_bytes<'a, 'b: 'a>(value: &'a Self::SelfType<'b>) -> Self::AsBytes<'a>
    where
        Self: 'b,
    {
        value.into_bytes()
    }

    fn type_name() -> TypeName {
        TypeName::new("memocp::Hash")
    }
}

impl Key for Hash {
    fn compare(data1: &[u8], data2: &[u8]) -> Ordering {
        data1.cmp(data2)
    }
}
