use anyhow::anyhow;
use clap::ValueEnum;
use std::fs;
use std::path::Path;
use tokio::task;
use uuid::Uuid;

#[derive(Debug, Clone, Copy, PartialEq, Eq, ValueEnum)]
pub enum CopyOp {
    HardLink,
    Reflink,
    Copy,
}

impl CopyOp {
    pub async fn execute(
        &self,
        source: impl AsRef<Path>,
        destination: impl AsRef<Path>,
        override_existing: bool,
    ) -> anyhow::Result<()> {
        task::spawn_blocking({
            let source = source.as_ref().to_path_buf();
            let destination = destination.as_ref().to_path_buf();
            let op = *self;
            move || {
                let _span = tracing::trace_span!("Copying file").entered();

                let Some(parent) = destination.parent() else {
                    return Err(anyhow!(
                        "Destination path has no parent directory, cannot create temporary file."
                    ));
                };

                fs::create_dir_all(parent)?;

                match op {
                    CopyOp::HardLink => {
                        fs::hard_link(&source, &destination)?;
                    }
                    CopyOp::Reflink | CopyOp::Copy => {
                        if !override_existing && fs::exists(&destination)? {
                            return Err(anyhow!("Destination file already exists."));
                        }

                        let temp_file_name = format!(".memocp_tmp_{}", Uuid::new_v4().simple());
                        let temp_path = parent.join(temp_file_name);

                        let result = match op {
                            CopyOp::Reflink => {
                                reflink_copy::reflink_or_copy(&source, &temp_path)?;
                                Ok(())
                            }
                            CopyOp::Copy => {
                                fs::copy(&source, &temp_path)?;
                                Ok(())
                            }
                            _ => unreachable!(),
                        };

                        if let Err(e) = result {
                            let _ = fs::remove_file(&temp_path);
                            return Err(e);
                        }

                        if let Err(e) = fs::rename(&temp_path, &destination) {
                            let _ = fs::remove_file(&temp_path);
                            return Err(e.into());
                        }
                    }
                }

                Ok::<(), anyhow::Error>(())
            }
        })
        .await??;

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_fs::prelude::*;
    use std::fs;

    #[tokio::test]
    async fn test_copy() -> anyhow::Result<()> {
        let temp = assert_fs::TempDir::new()?;
        let source_file = temp.child("source.txt");
        source_file.write_str("hello world")?;

        let dest_file = temp.child("dest.txt");

        CopyOp::Copy
            .execute(source_file.path(), dest_file.path(), false)
            .await?;

        assert!(dest_file.exists());
        assert_eq!(fs::read_to_string(dest_file.path())?, "hello world");

        Ok(())
    }

    #[tokio::test]
    async fn test_hard_link() -> anyhow::Result<()> {
        let temp = assert_fs::TempDir::new()?;
        let source_file = temp.child("source.txt");
        source_file.write_str("hello hardlink")?;

        let dest_file = temp.child("dest_hl.txt");

        CopyOp::HardLink
            .execute(source_file.path(), dest_file.path(), false)
            .await?;

        assert!(dest_file.exists());

        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;
            let source_meta = fs::metadata(source_file.path())?;
            let dest_meta = fs::metadata(dest_file.path())?;
            assert_eq!(source_meta.ino(), dest_meta.ino());
        }

        Ok(())
    }

    #[tokio::test]
    async fn test_reflink() -> anyhow::Result<()> {
        let temp = assert_fs::TempDir::new()?;
        let source_file = temp.child("source.txt");
        source_file.write_str("hello reflink")?;

        let dest_file = temp.child("dest_rl.txt");

        CopyOp::Reflink
            .execute(source_file.path(), dest_file.path(), false)
            .await?;

        assert!(dest_file.exists());
        assert_eq!(fs::read_to_string(dest_file.path())?, "hello reflink");

        Ok(())
    }

    #[tokio::test]
    async fn test_copy_non_existent_parent() -> anyhow::Result<()> {
        let temp = assert_fs::TempDir::new()?;
        let source_file = temp.child("source.txt");
        source_file.write_str("hello")?;

        let dest_file = temp.child("subdir/another/dest.txt");

        CopyOp::Copy
            .execute(source_file.path(), dest_file.path(), false)
            .await?;

        assert!(dest_file.exists());
        assert_eq!(fs::read_to_string(dest_file.path())?, "hello");

        Ok(())
    }

    #[tokio::test]
    async fn test_copy_override_true() -> anyhow::Result<()> {
        let temp = assert_fs::TempDir::new()?;
        let source_file = temp.child("source.txt");
        source_file.write_str("new content")?;

        let dest_file = temp.child("dest.txt");
        dest_file.write_str("old content")?;

        CopyOp::Copy
            .execute(source_file.path(), dest_file.path(), true)
            .await?;

        assert!(dest_file.exists());
        assert_eq!(fs::read_to_string(dest_file.path())?, "new content");

        Ok(())
    }

    #[tokio::test]
    async fn test_copy_override_false_exists() -> anyhow::Result<()> {
        let temp = assert_fs::TempDir::new()?;
        let source_file = temp.child("source.txt");
        source_file.write_str("new content")?;

        let dest_file = temp.child("dest.txt");
        dest_file.write_str("old content")?;

        let result = CopyOp::Copy
            .execute(source_file.path(), dest_file.path(), false)
            .await;

        assert!(result.is_err());
        assert_eq!(
            result.unwrap_err().to_string(),
            "Destination file already exists."
        );
        assert_eq!(fs::read_to_string(dest_file.path())?, "old content");

        Ok(())
    }

    #[tokio::test]
    async fn test_reflink_override_true() -> anyhow::Result<()> {
        let temp = assert_fs::TempDir::new()?;
        let source_file = temp.child("source.txt");
        source_file.write_str("new content reflink")?;

        let dest_file = temp.child("dest.txt");
        dest_file.write_str("old content")?;

        CopyOp::Reflink
            .execute(source_file.path(), dest_file.path(), true)
            .await?;

        assert!(dest_file.exists());
        assert_eq!(fs::read_to_string(dest_file.path())?, "new content reflink");

        Ok(())
    }

    #[tokio::test]
    async fn test_hard_link_override_false_exists() -> anyhow::Result<()> {
        let temp = assert_fs::TempDir::new()?;
        let source_file = temp.child("source.txt");
        source_file.write_str("new content")?;

        let dest_file = temp.child("dest.txt");
        dest_file.write_str("old content")?;

        let result = CopyOp::HardLink
            .execute(source_file.path(), dest_file.path(), false)
            .await;

        // Currently HardLink doesn't check for existence and fs::hard_link fails if dest exists
        assert!(result.is_err());
        assert_eq!(fs::read_to_string(dest_file.path())?, "old content");

        Ok(())
    }
}
