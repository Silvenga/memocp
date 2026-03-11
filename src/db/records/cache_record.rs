use crate::hashing::Hash;
use borsh::{BorshDeserialize, BorshSerialize};
use redb::{TypeName, Value};

#[derive(BorshSerialize, BorshDeserialize, Debug, PartialEq, Eq)]
pub struct CacheRecord {
    pub file_size_bytes: u64,
    pub file_modified_time: u128,
    pub file_created_time: u128,
    pub file_hash: Hash,
}

impl Value for CacheRecord {
    type SelfType<'a> = CacheRecord;
    type AsBytes<'a> = Vec<u8>;

    fn fixed_width() -> Option<usize> {
        None
    }

    fn from_bytes<'a>(data: &'a [u8]) -> Self::SelfType<'a>
    where
        Self: 'a,
    {
        borsh::from_slice(data).expect("Failed to deserialize SeenRecord")
    }

    fn as_bytes<'a, 'b: 'a>(value: &'a Self::SelfType<'b>) -> Self::AsBytes<'a>
    where
        Self: 'b,
    {
        borsh::to_vec(value).expect("Failed to serialize SeenRecord")
    }

    fn type_name() -> TypeName {
        TypeName::new("memocp::SeenRecord")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip() {
        let record = CacheRecord {
            file_size_bytes: 123,
            file_modified_time: 456,
            file_created_time: 789,
            file_hash: Hash::default(),
        };
        let serialized = CacheRecord::as_bytes(&record);
        let deserialized = CacheRecord::from_bytes(&serialized);
        assert_eq!(record, deserialized);
    }
}
