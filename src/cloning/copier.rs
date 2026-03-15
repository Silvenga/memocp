use crate::cloning::CopyOp;
use crate::db::Db;
use crate::models::FileMetadata;
use crate::templating::Templater;
use humantime::format_duration;
use std::path::Path;
use std::time::Instant;

pub struct Copier {
    db: Db,
    templater: Templater,
    copy_op: CopyOp,
    override_existing: bool,
}

impl Copier {
    pub fn new(db: &Db, templater: Templater) -> Self {
        Self {
            db: db.clone(),
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

    pub async fn try_copy(
        &self,
        file: impl AsRef<Path>,
        metadata: &FileMetadata,
    ) -> anyhow::Result<FileCopyResult> {
        if self.db.exists_seen(metadata.file_hash).await? {
            tracing::debug!(
                "File already seen with hash {}.",
                metadata.file_hash.as_string()
            );
            return Ok(FileCopyResult::Skipped);
        }

        let destination = self.templater.render_destination(&file, &metadata)?;

        tracing::debug!("Copying to {:?}.", destination);

        let copy_start = Instant::now();

        self.copy_op
            .execute(&file, &destination, self.override_existing)
            .await?;

        tracing::debug!("Copied after {}.", format_duration(copy_start.elapsed()));

        Ok(FileCopyResult::Copied)
    }
}

pub enum FileCopyResult {
    Skipped,
    Copied,
}
