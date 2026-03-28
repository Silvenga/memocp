use crate::db::migrations::get_migrations;
use crate::db::{CacheRecord, SeenRecord};
use crate::hashing::{Hash, Hasher};
use redb::{Database, ReadableDatabase, ReadableTable, ReadableTableMetadata, TableDefinition};
use std::ffi::OsStr;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use thiserror::Error;
use tokio::sync::mpsc;
use tokio::task;

const MIGRATIONS_TABLE: TableDefinition<u32, ()> = TableDefinition::new("v1_migrations");
const SEEN_TABLE: TableDefinition<Hash, SeenRecord> = TableDefinition::new("v1_seen");
const CACHE_TABLE: TableDefinition<Hash, CacheRecord> = TableDefinition::new("v2_cache");

#[derive(Clone)]
pub struct Db {
    db: Arc<Database>,
}

impl Db {
    pub async fn open(database: Database) -> anyhow::Result<Self> {
        let db = Self {
            db: database.into(),
        };
        db.migrate().await?;
        Ok(db)
    }

    pub async fn open_file(path: impl AsRef<Path>) -> anyhow::Result<Self> {
        tracing::debug!("Acquiring lock of state database at {:?}.", path.as_ref());
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
                if let Some(result) = table.get(hash_path(&path))? {
                    let record = result.value();
                    if record.file_size_bytes == file_size_bytes
                        && record.file_modified_time == file_modified_time
                        && record.file_created_time == file_created_time
                    {
                        let file_hash = record.file_hash;
                        tracing::debug!(
                            "[{path:?}]: Found matching source hash {}.",
                            file_hash.as_string()
                        );
                        return Ok(GetSourceHashResult::Hit { hash: file_hash });
                    }
                    tracing::debug!("[{path:?}]: File modified.");
                    return Ok(GetSourceHashResult::Modified);
                }
                tracing::debug!("[{path:?}]: No matching source hash found.");
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
                        hash_path(&path),
                        &CacheRecord {
                            file_size_bytes,
                            file_modified_time,
                            file_created_time,
                            file_hash,
                            file_path: path.as_os_str().as_encoded_bytes().to_vec(),
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
                    table.remove(hash_path(&path))?;
                }
                write_txn.commit()?;
                Ok(())
            }
        })
        .await?
    }

    pub async fn get_cached_paths(&self, tx: mpsc::Sender<PathBuf>) -> anyhow::Result<()> {
        task::spawn_blocking({
            let db = self.db.clone();
            move || -> anyhow::Result<()> {
                let read_txn = db.begin_read()?;
                let table = read_txn.open_table(CACHE_TABLE)?;
                for result in table.iter()? {
                    let (_, value) = result?;
                    let record = value.value();
                    // SAFETY: We know these bytes came from an OsStr via as_encoded_bytes
                    let os_str =
                        unsafe { OsStr::from_encoded_bytes_unchecked(record.file_path.as_slice()) };
                    let path = PathBuf::from(os_str);
                    if tx.blocking_send(path).is_err() {
                        break;
                    }
                }
                Ok(())
            }
        })
        .await?
    }

    pub async fn count_cached_paths(&self) -> anyhow::Result<u64> {
        task::spawn_blocking({
            let db = self.db.clone();
            move || -> anyhow::Result<u64> {
                let read_txn = db.begin_read()?;
                let table = read_txn.open_table(CACHE_TABLE)?;
                Ok(table.len()?)
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

    pub async fn migrate(&self) -> anyhow::Result<bool> {
        task::spawn_blocking({
            let db = self.db.clone();
            move || -> anyhow::Result<bool> {
                let write_txn = db.begin_write()?;
                {
                    let mut migrations_table = write_txn.open_table(MIGRATIONS_TABLE)?;

                    let needed_migrations = {
                        let mut needed_migrations = Vec::new();
                        let possible_migrations = get_migrations();
                        for migration in possible_migrations {
                            let missing = migrations_table.get(migration.version())?.is_none();
                            if missing {
                                needed_migrations.push(migration);
                            }
                        }
                        needed_migrations
                    };

                    if needed_migrations.is_empty() {
                        return Ok(false);
                    }

                    for migration in needed_migrations {
                        tracing::debug!("Running migration {}.", migration);
                        migration.up(&write_txn)?;
                        migrations_table.insert(migration.version(), &())?;
                    }
                }
                write_txn.commit()?;
                Ok(true)
            }
        })
        .await?
    }
}

fn hash_path(path: impl AsRef<Path>) -> Hash {
    Hasher::hash_bytes(path.as_ref().as_os_str().as_encoded_bytes())
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

    #[tokio::test]
    pub async fn when_cached_paths_exist_then_get_cached_paths_should_return_all_paths() {
        let db = Db::open_in_memory().await.unwrap();
        let path1 = path::absolute("/test_path_1").unwrap();
        let path2 = path::absolute("/test_path_2").unwrap();
        db.set_source_hash(&path1, 10, 20, 30, Hash::default())
            .await
            .unwrap();
        db.set_source_hash(&path2, 11, 22, 33, Hash::default())
            .await
            .unwrap();

        let (tx, mut rx) = mpsc::channel(10);
        db.get_cached_paths(tx).await.unwrap();

        let mut paths = Vec::new();
        while let Some(path) = rx.recv().await {
            paths.push(path);
        }

        assert_eq!(paths.len(), 2);
        assert!(paths.contains(&path1));
        assert!(paths.contains(&path2));
    }

    #[tokio::test]
    pub async fn when_cached_paths_exist_then_count_cached_paths_should_return_correct_count() {
        let db = Db::open_in_memory().await.unwrap();
        let path1 = path::absolute("/test_path_1").unwrap();
        let path2 = path::absolute("/test_path_2").unwrap();
        db.set_source_hash(&path1, 10, 20, 30, Hash::default())
            .await
            .unwrap();
        db.set_source_hash(&path2, 11, 22, 33, Hash::default())
            .await
            .unwrap();

        let count = db.count_cached_paths().await.unwrap();
        assert_eq!(count, 2);
    }

    #[tokio::test]
    pub async fn when_migrations_not_needed_then_migrate_should_return_false() {
        let db = Db::open_in_memory().await.unwrap();
        let migrated = db.migrate().await.unwrap();
        assert!(!migrated);
    }
}
