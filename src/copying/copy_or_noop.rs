use crate::copying::Copier;
use crate::copying::file_copy_result::FileCopyResult;
use crate::copying::get_current_time::get_current_time;
use crate::db::{Db, SeenRecord};
use crate::models::FileMetadata;
use bytesize::ByteSize;
use std::path::Path;

pub enum CopyOrNoop {
    Copy { copier: Copier },
    Noop { db: Db },
}

impl CopyOrNoop {
    pub fn new_copier(copier: Copier) -> Self {
        Self::Copy { copier }
    }

    pub fn new_noop(db: &Db) -> Self {
        Self::Noop { db: db.clone() }
    }

    pub async fn try_copy(
        &self,
        file: impl AsRef<Path>,
        metadata: &FileMetadata,
    ) -> anyhow::Result<FileCopyResult> {
        match self {
            CopyOrNoop::Copy { copier } => copier.try_copy(file, metadata).await,
            CopyOrNoop::Noop { db } => {
                if db.exists_seen(metadata.file_hash).await? {
                    tracing::debug!(
                        "File already seen with hash {}.",
                        metadata.file_hash.as_string()
                    );
                } else {
                    db.set_seen(
                        metadata.file_hash,
                        SeenRecord {
                            copied_time: get_current_time(),
                        },
                    )
                    .await?;

                    tracing::info!(
                        "Loaded {} ({}).",
                        file.as_ref().display(),
                        ByteSize::b(metadata.file_size_bytes),
                    );
                }
                Ok(FileCopyResult::Skipped)
            }
        }
    }
}
