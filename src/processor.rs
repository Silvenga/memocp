use crate::copying::{Copier, FileCopyResult};
use crate::db::{Db, GetSourceHashResult};
use crate::hashing::{Hash, Hasher};
use crate::models::FileMetadata;
use humantime::format_duration;
use std::path::Path;
use std::time::{Instant, UNIX_EPOCH};
use tokio::fs;

pub struct Processor {
    db: Db,
    hasher: Hasher,
    copier: Option<Copier>,
}

impl Processor {
    pub fn new(db: &Db, hasher: Hasher, copier: Option<Copier>) -> Self {
        Self {
            db: db.clone(),
            hasher,
            copier,
        }
    }

    pub async fn process(&self, file: impl AsRef<Path>) -> anyhow::Result<FileResult> {
        let (metadata, cache_result) = self.get_file_metadata(&file).await?;

        let copy_result = if let Some(copier) = &self.copier {
            copier.try_copy(&file, &metadata).await?
        } else {
            FileCopyResult::Skipped
        };

        Ok::<FileResult, anyhow::Error>(FileResult {
            metadata,
            copy_result,
            cache_result,
        })
    }

    async fn get_file_metadata(
        &self,
        file: impl AsRef<Path>,
    ) -> anyhow::Result<(FileMetadata, FileCacheResult)> {
        let metadata = fs::metadata(&file).await?;

        let file_size_bytes = metadata.len();
        let file_created_time = match metadata.created() {
            Ok(time) => time.duration_since(UNIX_EPOCH)?.as_nanos(),
            Err(_) => 0,
        };
        let file_modified_time = metadata.modified()?.duration_since(UNIX_EPOCH)?.as_nanos();
        let (file_hash, cache_state) = self
            .get_or_read_file_hash(
                &file,
                file_size_bytes,
                file_modified_time,
                file_created_time,
            )
            .await?;

        let metadata = FileMetadata {
            file_size_bytes,
            file_modified_time,
            file_created_time,
            file_hash,
        };

        Ok((metadata, cache_state))
    }

    async fn get_or_read_file_hash(
        &self,
        file: impl AsRef<Path>,
        file_size_bytes: u64,
        file_modified_time: u128,
        file_created_time: u128,
    ) -> anyhow::Result<(Hash, FileCacheResult)> {
        let cache_result = self
            .db
            .try_get_source_hash(
                &file,
                file_size_bytes,
                file_modified_time,
                file_created_time,
            )
            .await?;

        match cache_result {
            GetSourceHashResult::Hit { hash } => Ok((hash, FileCacheResult::Unchanged)),
            GetSourceHashResult::Modified | GetSourceHashResult::Miss => {
                if matches!(cache_result, GetSourceHashResult::Modified) {
                    tracing::debug!("File was modified since last seen, recalculating hash.");
                } else {
                    tracing::debug!("File not seen before, calculating hash.");
                }

                let hashing_start = Instant::now();

                let file_hash = self.hasher.hash_file(&file).await?;

                tracing::debug!(
                    "Calculated hash {} after {}.",
                    file_hash.as_string(),
                    format_duration(hashing_start.elapsed())
                );

                self.db
                    .set_source_hash(
                        &file,
                        file_size_bytes,
                        file_modified_time,
                        file_created_time,
                        file_hash,
                    )
                    .await?;

                if matches!(cache_result, GetSourceHashResult::Modified) {
                    Ok((file_hash, FileCacheResult::Modified))
                } else {
                    Ok((file_hash, FileCacheResult::New))
                }
            }
        }
    }
}

pub enum FileCacheResult {
    New,
    Modified,
    Unchanged,
}

pub struct FileResult {
    pub metadata: FileMetadata,
    pub cache_result: FileCacheResult,
    pub copy_result: FileCopyResult,
}
