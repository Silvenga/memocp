use crate::db::{CacheRecord, SeenRecord};
use crate::scanning::hashing::Hash;
use redb::{Database, ReadableDatabase, TableDefinition};
use std::sync::Arc;
use tokio::task;
use tracing::debug;

const SEEN_TABLE: TableDefinition<&str, SeenRecord> = TableDefinition::new("v1_seen");
const CACHE_TABLE: TableDefinition<&str, CacheRecord> = TableDefinition::new("v1_cache");

#[derive(Clone)]
pub struct Db {
    db: Arc<Database>,
}

impl Db {
    pub async fn open(database: Database) -> anyhow::Result<Self> {
        let db = Self {
            db: database.into(),
        };
        db.create_tables().await?;
        Ok(db)
    }

    pub async fn open_file(path: String) -> anyhow::Result<Self> {
        debug!("Acquiring lock of state database at {}.", path);
        Self::open(Database::create(path)?).await
    }

    #[cfg(test)]
    pub async fn open_in_memory() -> anyhow::Result<Self> {
        let db = Database::builder().create_with_backend(redb::backends::InMemoryBackend::new())?;
        Self::open(db).await
    }

    pub async fn try_get_source_hash(
        &self,
        path: impl Into<String>,
        file_size_bytes: u64,
        file_modified_time: u64,
        file_created_time: u64,
    ) -> anyhow::Result<Option<Hash>> {
        task::spawn_blocking({
            let db = self.db.clone();
            let path = path.into();
            move || -> anyhow::Result<Option<Hash>> {
                let read_txn = db.begin_read()?;
                let table = read_txn.open_table(CACHE_TABLE)?;
                if let Some(result) = table.get(path.as_str())? {
                    let record = result.value();
                    if record.file_size_bytes == file_size_bytes
                        && record.file_modified_time == file_modified_time
                        && record.file_created_time == file_created_time
                    {
                        let file_hash = record.file_hash;
                        // PERF: as_string when debugging disabled?
                        debug!(
                            "[{}]: Found matching source hash {}.",
                            path,
                            file_hash.as_string()
                        );
                        return Ok(Some(file_hash));
                    }
                }
                debug!("[{}]: No matching source hash found.", path);
                Ok(None)
            }
        })
        .await?
    }

    pub async fn set_source_hash(
        &self,
        path: impl Into<String>,
        file_size_bytes: u64,
        file_modified_time: u64,
        file_created_time: u64,
        file_hash: Hash,
    ) -> anyhow::Result<()> {
        task::spawn_blocking({
            let db = self.db.clone();
            let path = path.into();
            move || -> anyhow::Result<()> {
                let write_txn = db.begin_write()?;
                {
                    let mut table = write_txn.open_table(CACHE_TABLE)?;
                    table.insert(
                        path.as_str(),
                        &CacheRecord {
                            file_size_bytes,
                            file_modified_time,
                            file_created_time,
                            file_hash,
                        },
                    )?;
                }
                write_txn.commit()?;
                Ok(())
            }
        })
        .await?
    }

    pub async fn remove_source_hash(&self, path: impl Into<String>) -> anyhow::Result<()> {
        task::spawn_blocking({
            let db = self.db.clone();
            let path = path.into();
            move || -> anyhow::Result<()> {
                let write_txn = db.begin_write()?;
                {
                    let mut table = write_txn.open_table(CACHE_TABLE)?;
                    table.remove(path.as_str())?;
                }
                write_txn.commit()?;
                Ok(())
            }
        })
        .await?
    }

    pub async fn create_tables(&self) -> anyhow::Result<()> {
        task::spawn_blocking({
            let db = self.db.clone();
            move || -> anyhow::Result<()> {
                let write_txn = db.begin_write()?;
                write_txn.open_table(SEEN_TABLE)?;
                write_txn.open_table(CACHE_TABLE)?;
                write_txn.commit()?;
                Ok(())
            }
        })
        .await?
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_matches::assert_matches;

    #[tokio::test]
    pub async fn when_hash_doesnt_exist_then_try_get_source_hash_should_return_none() {
        let db = Db::open_in_memory().await.unwrap();
        let path = "test_path";
        let result = db.try_get_source_hash(path, 10, 20, 30).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    pub async fn when_hash_exists_then_try_get_source_hash_should_return_some() {
        let db = Db::open_in_memory().await.unwrap();
        let path = "test_path";
        db.set_source_hash(path, 10, 20, 30, Hash::default())
            .await
            .unwrap();
        let result = db.try_get_source_hash(path, 10, 20, 30).await.unwrap();
        assert_matches!(result, Some(hash) if hash == Hash::default());
    }

    #[tokio::test]
    pub async fn when_hash_exists_then_remove_source_hash_should_remove_hash() {
        let db = Db::open_in_memory().await.unwrap();
        let path = "test_path";
        db.set_source_hash(path, 10, 20, 30, Hash::default())
            .await
            .unwrap();
        db.remove_source_hash(path).await.unwrap();
        let result = db.try_get_source_hash(path, 10, 20, 30).await.unwrap();
        assert_matches!(result, None);
    }
}
