use crate::db::Db;
use crate::file_metadata::FileMetadata;
use crate::hashing::{Hash, Hasher};
use crate::templating::Templater;
use std::path::Path;
use std::time::UNIX_EPOCH;
use tokio::fs;

pub struct Copier {
    db: Db,
    hasher: Hasher,
    templater: Templater,
}

impl Copier {
    pub fn new(db: Db, hasher: Hasher, templater: Templater) -> Self {
        Self {
            db,
            hasher,
            templater,
        }
    }

    pub async fn copy(&self, file: impl AsRef<Path>) -> anyhow::Result<()> {
        let metadata = self.get_file_metadata(&file).await?;

        if self.db.exists_seen(metadata.file_hash).await? {
            tracing::debug!(
                "File {} already seen with hash {}.",
                file.as_ref().to_string_lossy(),
                metadata.file_hash.as_string()
            );
            return Ok(());
        }

        let destination = self.templater.render_destination(&file, &metadata)?;

        tracing::info!("Would have copied file to {:?}.", destination);

        Ok(())
    }

    async fn get_file_metadata(&self, file: impl AsRef<Path>) -> anyhow::Result<FileMetadata> {
        let metadata = fs::metadata(&file).await?;

        let file_size_bytes = metadata.len();
        let file_created_time = match metadata.created() {
            Ok(time) => time.duration_since(UNIX_EPOCH)?.as_nanos(),
            Err(_) => 0,
        };
        let file_modified_time = metadata.modified()?.duration_since(UNIX_EPOCH)?.as_nanos();
        let file_hash = self
            .get_or_read_file_hash(
                &file,
                file_size_bytes,
                file_modified_time,
                file_created_time,
            )
            .await?;

        Ok(FileMetadata {
            file_size_bytes,
            file_modified_time,
            file_created_time,
            file_hash,
        })
    }

    async fn get_or_read_file_hash(
        &self,
        file: impl AsRef<Path>,
        file_size_bytes: u64,
        file_modified_time: u128,
        file_created_time: u128,
    ) -> anyhow::Result<Hash> {
        if let Some(file_hash) = self
            .db
            .try_get_source_hash(
                &file,
                file_size_bytes,
                file_modified_time,
                file_created_time,
            )
            .await?
        {
            return Ok(file_hash);
        }

        let file_hash = self.hasher.hash_file(&file).await?;

        self.db
            .set_source_hash(
                &file,
                file_size_bytes,
                file_modified_time,
                file_created_time,
                file_hash,
            )
            .await?;

        Ok(file_hash)
    }
}
