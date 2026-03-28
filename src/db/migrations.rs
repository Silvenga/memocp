use crate::db::{CacheRecord, SeenRecord};
use crate::hashing::Hash;
use redb::{TableDefinition, TypeName, Value, WriteTransaction};
use std::fmt::{Display, Formatter};

const V1_SEEN_TABLE: TableDefinition<Hash, SeenRecord> = TableDefinition::new("v1_seen");
const V1_CACHE_TABLE: TableDefinition<&[u8], V1CacheRecord> = TableDefinition::new("v1_cache");
const V2_CACHE_TABLE: TableDefinition<Hash, CacheRecord> = TableDefinition::new("v2_cache");

pub fn get_migrations() -> Vec<Migration> {
    vec![
        Migration::new(1, |txn| {
            txn.open_table(V1_SEEN_TABLE)?;
            txn.open_table(V1_CACHE_TABLE)?;

            Ok(())
        }),
        Migration::new(2, |txn| {
            txn.delete_table(V1_CACHE_TABLE)?;
            txn.open_table(V2_CACHE_TABLE)?;

            Ok(())
        }),
    ]
}

type UpFn = Box<dyn Fn(&WriteTransaction) -> anyhow::Result<()>>;

pub struct Migration {
    version: u32,
    up: UpFn,
}

impl Migration {
    pub fn new(
        version: u32,
        up: impl Fn(&WriteTransaction) -> anyhow::Result<()> + 'static,
    ) -> Self {
        Self {
            version,
            up: Box::new(up),
        }
    }

    pub fn version(&self) -> u32 {
        self.version
    }

    pub fn up(&self, txn: &WriteTransaction) -> anyhow::Result<()> {
        (self.up)(txn)
    }
}

impl Display for Migration {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        write!(f, "V{}", self.version)
    }
}

#[derive(Debug)]
struct V1CacheRecord;

impl Value for V1CacheRecord {
    type SelfType<'a> = V1CacheRecord;
    type AsBytes<'a> = Vec<u8>;

    fn fixed_width() -> Option<usize> {
        None
    }

    fn from_bytes<'a>(_data: &'a [u8]) -> Self::SelfType<'a>
    where
        Self: 'a,
    {
        unreachable!()
    }

    fn as_bytes<'a, 'b: 'a>(_value: &'a Self::SelfType<'b>) -> Self::AsBytes<'a>
    where
        Self: 'b,
    {
        unreachable!()
    }

    fn type_name() -> TypeName {
        TypeName::new("memocp::SeenRecord")
    }
}
