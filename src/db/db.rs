use crate::db::{CacheRecord, SeenRecord};
use crate::hashing::Hash;
use redb::{Database, ReadableDatabase, TableDefinition};
use std::path::Path;
use std::sync::Arc;
use thiserror::Error;
use tokio::task;
use tracing::debug;

const SEEN_TABLE: TableDefinition<Hash, SeenRecord> = TableDefinition::new("v1_seen");
const CACHE_TABLE: TableDefinition<&[u8], CacheRecord> = TableDefinition::new("v1_cache");

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

    pub async fn open_file(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        debug!("Acquiring lock of state database at {:?}.", path.as_ref());
        Self::open(Database::create(path)?).await
    }

    #[cfg(test)]
    pub async fn open_in_memory() -> anyhow::Result<Self> {
        let db = Database::builder().create_with_backend(redb::backends::InMemoryBackend::new())?;
        Self::open(db).await
    }

    pub async fn try_get_source_hash(
        &self,
        path: impl AsRef<Path>,
        file_size_bytes: u64,
        file_modified_time: u128,
        file_created_time: u128,
    ) -> anyhow::Result<GetSourceHashResult> {
        if !path.as_ref().is_absolute() {
            return Err(DbError::PathMustBeAbsolute.into());
        }
        task::spawn_blocking({
            let db = self.db.clone();
            let path = path.as_ref().to_owned();
            move || -> anyhow::Result<GetSourceHashResult> {
                let read_txn = db.begin_read()?;
                let table = read_txn.open_table(CACHE_TABLE)?;
                if let Some(result) = table.get(path.as_os_str().as_encoded_bytes())? {
                    let record = result.value();
                    if record.file_size_bytes == file_size_bytes
                        && record.file_modified_time == file_modified_time
                        && record.file_created_time == file_created_time
                    {
                        let file_hash = record.file_hash;
                        debug!(
                            "[{path:?}]: Found matching source hash {}.",
                            file_hash.as_string()
                        );
                        return Ok(GetSourceHashResult::Hit { hash: file_hash });
                    }
                    debug!("[{path:?}]: File modified.");
                    return Ok(GetSourceHashResult::Modified);
                }
                debug!("[{path:?}]: No matching source hash found.");
                Ok(GetSourceHashResult::Miss)
            }
        })
        .await?
    }

    pub async fn set_source_hash(
        &self,
        path: impl AsRef<Path>,
        file_size_bytes: u64,
        file_modified_time: u128,
        file_created_time: u128,
        file_hash: Hash,
    ) -> anyhow::Result<()> {
        task::spawn_blocking({
            let db = self.db.clone();
            let path = path.as_ref().to_owned();
            move || -> anyhow::Result<()> {
                let write_txn = db.begin_write()?;
                {
                    let mut table = write_txn.open_table(CACHE_TABLE)?;
                    table.insert(
                        path.as_os_str().as_encoded_bytes(),
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

    pub async fn remove_source_hash(&self, path: impl AsRef<Path>) -> anyhow::Result<()> {
        task::spawn_blocking({
            let db = self.db.clone();
            let path = path.as_ref().to_owned();
            move || -> anyhow::Result<()> {
                let write_txn = db.begin_write()?;
                {
                    let mut table = write_txn.open_table(CACHE_TABLE)?;
                    table.remove(path.as_os_str().as_encoded_bytes())?;
                }
                write_txn.commit()?;
                Ok(())
            }
        })
        .await?
    }

    #[allow(dead_code)]
    pub async fn get_seen(&self, hash: Hash) -> anyhow::Result<Option<SeenRecord>> {
        task::spawn_blocking({
            let db = self.db.clone();
            move || -> anyhow::Result<Option<SeenRecord>> {
                let read_txn = db.begin_read()?;
                let table = read_txn.open_table(SEEN_TABLE)?;
                if let Some(result) = table.get(hash)? {
                    return Ok(Some(result.value()));
                }
                Ok(None)
            }
        })
        .await?
    }

    pub async fn exists_seen(&self, hash: Hash) -> anyhow::Result<bool> {
        task::spawn_blocking({
            let db = self.db.clone();
            move || -> anyhow::Result<bool> {
                let read_txn = db.begin_read()?;
                let table = read_txn.open_table(SEEN_TABLE)?;
                Ok(table.get(hash)?.is_some())
            }
        })
        .await?
    }

    pub async fn set_seen(&self, file_hash: Hash, record: SeenRecord) -> anyhow::Result<()> {
        task::spawn_blocking({
            let db = self.db.clone();
            move || -> anyhow::Result<()> {
                let write_txn = db.begin_write()?;
                {
                    let mut table = write_txn.open_table(SEEN_TABLE)?;
                    table.insert(file_hash, &record)?;
                }
                write_txn.commit()?;
                Ok(())
            }
        })
        .await?
    }

    #[allow(dead_code)]
    pub async fn remove_seen(&self, hash: Hash) -> anyhow::Result<()> {
        task::spawn_blocking({
            let db = self.db.clone();
            move || -> anyhow::Result<()> {
                let write_txn = db.begin_write()?;
                {
                    let mut table = write_txn.open_table(SEEN_TABLE)?;
                    table.remove(hash)?;
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

#[derive(Error, Debug)]
pub enum DbError {
    #[error("Path must be absolute.")]
    PathMustBeAbsolute,
}

#[derive(Debug)]
pub enum GetSourceHashResult {
    Hit { hash: Hash },
    Modified,
    Miss,
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_matches::assert_matches;
    use std::path;

    #[tokio::test]
    pub async fn when_hash_doesnt_exist_then_try_get_source_hash_should_return_miss() {
        let db = Db::open_in_memory().await.unwrap();
        let path = path::absolute("/test_path").unwrap();
        let result = db.try_get_source_hash(&path, 10, 20, 30).await.unwrap();
        assert_matches!(result, GetSourceHashResult::Miss);
    }

    #[tokio::test]
    pub async fn when_hash_exists_then_try_get_source_hash_should_return_hit() {
        let db = Db::open_in_memory().await.unwrap();
        let path = path::absolute("/test_path").unwrap();
        db.set_source_hash(&path, 10, 20, 30, Hash::default())
            .await
            .unwrap();
        let result = db.try_get_source_hash(&path, 10, 20, 30).await.unwrap();
        assert_matches!(result, GetSourceHashResult::Hit { hash } if hash == Hash::default());
    }

    #[tokio::test]
    pub async fn when_hash_exists_but_for_wrong_data_then_try_get_source_hash_should_return_modified()
     {
        let db = Db::open_in_memory().await.unwrap();
        let path = path::absolute("/test_path").unwrap();
        db.set_source_hash(&path, 10, 20, 30, Hash::default())
            .await
            .unwrap();
        let result = db.try_get_source_hash(&path, 11, 22, 33).await.unwrap();
        assert_matches!(result, GetSourceHashResult::Modified);
    }

    #[tokio::test]
    pub async fn when_hash_exists_then_remove_source_hash_should_remove_hash() {
        let db = Db::open_in_memory().await.unwrap();
        let path = path::absolute("/test_path").unwrap();
        db.set_source_hash(&path, 10, 20, 30, Hash::default())
            .await
            .unwrap();
        db.remove_source_hash(&path).await.unwrap();
        let result = db.try_get_source_hash(&path, 10, 20, 30).await.unwrap();
        assert_matches!(result, GetSourceHashResult::Miss);
    }

    #[tokio::test]
    pub async fn when_seen_doesnt_exist_then_get_seen_should_return_none() {
        let db = Db::open_in_memory().await.unwrap();
        let hash = Hash::default();
        let result = db.get_seen(hash).await.unwrap();
        assert!(result.is_none());
    }

    #[tokio::test]
    pub async fn when_seen_exists_then_get_seen_should_return_some() {
        let db = Db::open_in_memory().await.unwrap();
        let hash = Hash::default();
        let record = SeenRecord {
            copied_time: 101112,
        };
        db.set_seen(Hash::empty_hash(), record.clone())
            .await
            .unwrap();
        let result = db.get_seen(hash).await.unwrap();
        assert_eq!(result, Some(record));
    }

    #[tokio::test]
    pub async fn when_seen_exists_then_exists_seen_should_return_true() {
        let db = Db::open_in_memory().await.unwrap();
        let hash = Hash::default();
        let record = SeenRecord {
            copied_time: 101112,
        };
        db.set_seen(Hash::empty_hash(), record).await.unwrap();
        let result = db.exists_seen(hash).await.unwrap();
        assert!(result);
    }

    #[tokio::test]
    pub async fn when_seen_exists_then_remove_seen_should_remove_record() {
        let db = Db::open_in_memory().await.unwrap();
        let hash = Hash::default();
        let record = SeenRecord {
            copied_time: 101112,
        };
        db.set_seen(Hash::empty_hash(), record).await.unwrap();
        db.remove_seen(hash).await.unwrap();
        let result = db.get_seen(hash).await.unwrap();
        assert!(result.is_none());
    }
}
