use borsh::{BorshDeserialize, BorshSerialize};
use redb::{TypeName, Value};

#[derive(BorshSerialize, BorshDeserialize, Debug, Clone, PartialEq, Eq)]
pub struct SeenRecord {
    pub copied_time: u128,
}

impl Value for SeenRecord {
    type SelfType<'a> = SeenRecord;
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
        let record = SeenRecord {
            copied_time: 101112,
        };
        let serialized = SeenRecord::as_bytes(&record);
        let deserialized = SeenRecord::from_bytes(&serialized);
        assert_eq!(record, deserialized);
    }
}
