use crate::cloning::copy_op::CopyOp;
use crate::db::{Db, GetSourceHashResult};
use crate::file_metadata::FileMetadata;
use crate::hashing::{Hash, Hasher};
use crate::templating::Templater;
use humantime::format_duration;
use std::path::Path;
use std::time::{Instant, UNIX_EPOCH};
use tokio::fs;

pub struct Copier {
    db: Db,
    hasher: Hasher,
    templater: Templater,
    copy_op: CopyOp,
    override_existing: bool,
}

impl Copier {
    pub fn new(db: Db, hasher: Hasher, templater: Templater) -> Self {
        Self {
            db,
            hasher,
            templater,
            copy_op: CopyOp::Reflink,
            override_existing: false,
        }
    }

    pub fn with_copy_op(mut self, copy_op: CopyOp) -> Self {
        self.copy_op = copy_op;
        self
    }

    pub fn with_override_existing(mut self, override_existing: bool) -> Self {
        self.override_existing = override_existing;
        self
    }

    pub async fn copy(&self, file: impl AsRef<Path>) -> anyhow::Result<CopyResult> {
        let (metadata, cache_result) = self.get_file_metadata(&file).await?;

        if self.db.exists_seen(metadata.file_hash).await? {
            tracing::debug!(
                "File already seen with hash {}.",
                metadata.file_hash.as_string()
            );
            return Ok(CopyResult {
                metadata,
                copy_result: FileCopyResult::Skipped,
                cache_result,
            });
        }

        let destination = self.templater.render_destination(&file, &metadata)?;

        tracing::debug!("Copying to {:?}.", destination);

        let copy_start = Instant::now();

        self.copy_op
            .execute(&file, &destination, self.override_existing)
            .await?;

        tracing::debug!("Copied after {}.", format_duration(copy_start.elapsed()));

        Ok::<CopyResult, anyhow::Error>(CopyResult {
            metadata,
            copy_result: FileCopyResult::Copied,
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

pub enum FileCopyResult {
    Skipped,
    Copied,
}

pub struct CopyResult {
    pub metadata: FileMetadata,
    pub copy_result: FileCopyResult,
    pub cache_result: FileCacheResult,
}
