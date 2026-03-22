use crate::db::Db;
use crate::progress::CleanupProgress;
use futures::StreamExt;
use humantime::format_duration;
use num_format::{Locale, ToFormattedString};
use std::path::PathBuf;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Instant;
use tokio::fs;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tracing::Instrument;

pub struct Cleanup {
    db: Db,
    queue_depth: usize,
    concurrency: usize,
}

impl Cleanup {
    pub fn new(db: &Db) -> Self {
        Cleanup {
            db: db.clone(),
            queue_depth: 10_000,
            concurrency: 8,
        }
    }

    pub fn with_concurrency(mut self, concurrency: usize) -> Self {
        self.concurrency = concurrency;
        self
    }

    pub fn with_queue_depth(mut self, queue_depth: usize) -> Self {
        self.queue_depth = queue_depth;
        self
    }

    pub async fn cleanup(&self) -> anyhow::Result<()> {
        let total = self.db.count_cached_paths().await?;
        let (progress, span) = CleanupProgress::new(total);

        let (tx, rx) = mpsc::channel::<PathBuf>(self.queue_depth);

        let start_time = Instant::now();
        let checked_count = Arc::new(AtomicU64::new(0));
        let removed_count = Arc::new(AtomicU64::new(0));

        let scan_task = self.db.get_cached_paths(tx);

        let check_task = ReceiverStream::new(rx).for_each_concurrent(self.concurrency, |path| {
            let db = self.db.clone();
            let progress = progress.clone();
            let checked_count = checked_count.clone();
            let removed_count = removed_count.clone();
            async move {
                tracing::debug!("Checking file {:?}.", path);
                match fs::try_exists(&path).await {
                    Ok(true) => {
                        // File exists, keep it.
                    }
                    Ok(false) => {
                        tracing::debug!("Removing non-existent file from cache {:?}.", path);
                        if let Err(e) = db.remove_source_hash(&path).await {
                            tracing::warn!(
                                "Failed to remove source hash for {:?}: {}",
                                path.display(),
                                e
                            );
                        } else {
                            progress.inc_removed();
                            removed_count.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                    Err(e) => {
                        tracing::warn!(
                            "Failed to check if file exists {:?}: {}",
                            path.display(),
                            e
                        );
                    }
                }
                progress.inc_checked();
                checked_count.fetch_add(1, Ordering::Relaxed);
            }
        });

        let (scan_task, _) = tokio::join!(scan_task, check_task.instrument(span));
        scan_task?;

        tracing::info!(
            "Database cleanup complete, checked {} entries, removed {} non-existent files in {}.",
            checked_count
                .load(Ordering::Relaxed)
                .to_formatted_string(&Locale::en),
            removed_count
                .load(Ordering::Relaxed)
                .to_formatted_string(&Locale::en),
            format_duration(start_time.elapsed())
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::hashing::Hash;
    use assert_fs::prelude::*;
    use assert_matches::assert_matches;

    #[tokio::test]
    async fn when_files_do_not_exist_then_cleanup_should_remove_from_db() {
        let db = Db::open_in_memory().await.unwrap();
        let temp = assert_fs::TempDir::new().unwrap();
        let file1 = temp.child("file1");
        file1.touch().unwrap();
        let file2 = temp.child("file2");
        file2.touch().unwrap();

        let path1 = file1.path().to_path_buf();
        let path2 = file2.path().to_path_buf();

        db.set_source_hash(&path1, 0, 0, 0, Hash::default())
            .await
            .unwrap();
        db.set_source_hash(&path2, 0, 0, 0, Hash::default())
            .await
            .unwrap();

        std::fs::remove_file(&path1).unwrap();

        let cleanup = Cleanup::new(&db);
        cleanup.cleanup().await.unwrap();

        let result1 = db.try_get_source_hash(&path1, 0, 0, 0).await.unwrap();
        assert_matches!(result1, crate::db::GetSourceHashResult::Miss);

        let result2 = db.try_get_source_hash(&path2, 0, 0, 0).await.unwrap();
        assert_matches!(result2, crate::db::GetSourceHashResult::Hit { .. });
    }
}
