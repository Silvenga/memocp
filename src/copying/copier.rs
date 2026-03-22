use crate::copying::CopyOp;
use crate::copying::file_copy_result::FileCopyResult;
use crate::copying::get_current_time::get_current_time;
use crate::db::{Db, SeenRecord};
use crate::models::FileMetadata;
use crate::templating::Templater;
use bytesize::ByteSize;
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

        let destination = self.templater.render_destination(&file, metadata)?;

        tracing::debug!("Copying to {:?}.", destination);

        let copy_start = Instant::now();

        self.copy_op
            .execute(&file, &destination, self.override_existing)
            .await?;

        self.db
            .set_seen(
                metadata.file_hash,
                SeenRecord {
                    copied_time: get_current_time(),
                },
            )
            .await?;

        tracing::info!(
            "Copied {} ({}) after {}.",
            file.as_ref().display(),
            ByteSize::b(metadata.file_size_bytes),
            format_duration(copy_start.elapsed())
        );

        Ok(FileCopyResult::Copied)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hashing::Hash;
    use assert_fs::TempDir;
    use assert_fs::prelude::*;
    use assert_matches::assert_matches;
    use std::str::FromStr;

    #[tokio::test]
    async fn when_file_not_seen_then_try_copy_should_copy_file() {
        let temp = TempDir::new().unwrap();
        let source_dir = temp.child("source");
        source_dir.create_dir_all().unwrap();
        let dest_dir = temp.child("dest");
        dest_dir.create_dir_all().unwrap();
        let file = source_dir.child("foo.txt");
        file.write_str("hello world").unwrap();
        let db = Db::open_in_memory().await.unwrap();
        let templater = Templater::new(source_dir.path(), dest_dir.path().to_string_lossy());
        let copier = Copier::new(&db, templater).with_copy_op(CopyOp::Copy);
        let metadata = FileMetadata {
            file_size_bytes: 11,
            file_modified_time: 0,
            file_created_time: 0,
            file_hash: Hash::from_str(
                "d74981efa70a0c880b8d8c1985d075dbcbf679b99a5f9914e5aaf96b831a9e24",
            )
            .unwrap(),
        };

        let result = copier.try_copy(file.path(), &metadata).await.unwrap();

        assert_matches!(result, FileCopyResult::Copied);
        assert!(dest_dir.child("foo.txt").exists());
        assert!(db.exists_seen(metadata.file_hash).await.unwrap());
    }

    #[tokio::test]
    async fn when_file_already_seen_then_try_copy_should_skip_file() {
        let temp = TempDir::new().unwrap();
        let source_dir = temp.child("source");
        source_dir.create_dir_all().unwrap();
        let dest_dir = temp.child("dest");
        dest_dir.create_dir_all().unwrap();
        let file = source_dir.child("foo.txt");
        file.write_str("hello world").unwrap();
        let db = Db::open_in_memory().await.unwrap();
        let hash =
            Hash::from_str("d74981efa70a0c880b8d8c1985d075dbcbf679b99a5f9914e5aaf96b831a9e24")
                .unwrap();
        db.set_seen(hash, SeenRecord { copied_time: 0 })
            .await
            .unwrap();
        let templater = Templater::new(source_dir.path(), dest_dir.path().to_string_lossy());
        let copier = Copier::new(&db, templater).with_copy_op(CopyOp::Copy);
        let metadata = FileMetadata {
            file_size_bytes: 11,
            file_modified_time: 0,
            file_created_time: 0,
            file_hash: hash,
        };

        let result = copier.try_copy(file.path(), &metadata).await.unwrap();

        assert_matches!(result, FileCopyResult::Skipped);
        assert!(!dest_dir.child("foo.txt").exists());
    }

    #[tokio::test]
    async fn when_override_is_false_and_destination_exists_then_try_copy_should_error() {
        let temp = TempDir::new().unwrap();
        let source_dir = temp.child("source");
        source_dir.create_dir_all().unwrap();
        let dest_dir = temp.child("dest");
        dest_dir.create_dir_all().unwrap();
        let file = source_dir.child("foo.txt");
        file.write_str("hello world").unwrap();
        let existing_dest = dest_dir.child("foo.txt");
        existing_dest.write_str("existing").unwrap();
        let db = Db::open_in_memory().await.unwrap();
        let templater = Templater::new(source_dir.path(), dest_dir.path().to_string_lossy());
        let copier = Copier::new(&db, templater)
            .with_copy_op(CopyOp::Copy)
            .with_override_existing(false);
        let metadata = FileMetadata {
            file_size_bytes: 11,
            file_modified_time: 0,
            file_created_time: 0,
            file_hash: Hash::from_str(
                "d74981efa70a0c880b8d8c1985d075dbcbf679b99a5f9914e5aaf96b831a9e24",
            )
            .unwrap(),
        };

        let result = copier.try_copy(file.path(), &metadata).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn when_override_is_true_and_destination_exists_then_try_copy_should_copy_file() {
        let temp = TempDir::new().unwrap();
        let source_dir = temp.child("source");
        source_dir.create_dir_all().unwrap();
        let dest_dir = temp.child("dest");
        dest_dir.create_dir_all().unwrap();
        let file = source_dir.child("foo.txt");
        file.write_str("hello world").unwrap();
        let existing_dest = dest_dir.child("foo.txt");
        existing_dest.write_str("existing").unwrap();
        let db = Db::open_in_memory().await.unwrap();
        let templater = Templater::new(source_dir.path(), dest_dir.path().to_string_lossy());
        let copier = Copier::new(&db, templater)
            .with_copy_op(CopyOp::Copy)
            .with_override_existing(true);
        let metadata = FileMetadata {
            file_size_bytes: 11,
            file_modified_time: 0,
            file_created_time: 0,
            file_hash: Hash::from_str(
                "d74981efa70a0c880b8d8c1985d075dbcbf679b99a5f9914e5aaf96b831a9e24",
            )
            .unwrap(),
        };

        let result = copier.try_copy(file.path(), &metadata).await.unwrap();

        assert_matches!(result, FileCopyResult::Copied);
        assert_eq!(
            std::fs::read_to_string(existing_dest.path()).unwrap(),
            "hello world"
        );
    }

    #[tokio::test]
    async fn when_new_called_then_it_should_have_default_values() {
        let db = Db::open_in_memory().await.unwrap();
        let templater = Templater::new("/source", "/dest");

        let copier = Copier::new(&db, templater);

        assert_matches!(copier.copy_op, CopyOp::Reflink);
        assert!(!copier.override_existing);
    }

    #[tokio::test]
    async fn when_with_copy_op_called_then_it_should_set_copy_op() {
        let db = Db::open_in_memory().await.unwrap();
        let templater = Templater::new("/source", "/dest");

        let copier = Copier::new(&db, templater).with_copy_op(CopyOp::Copy);

        assert_matches!(copier.copy_op, CopyOp::Copy);
    }

    #[tokio::test]
    async fn when_with_override_existing_called_then_it_should_set_override_existing() {
        let db = Db::open_in_memory().await.unwrap();
        let templater = Templater::new("/source", "/dest");

        let copier = Copier::new(&db, templater).with_override_existing(true);

        assert!(copier.override_existing);
    }
}
