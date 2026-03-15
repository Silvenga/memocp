use crate::cloning::{CopyResult, FileCacheResult, FileCopyResult};
use std::sync::atomic::{AtomicU64, Ordering};

#[derive(Default)]
pub struct CopyStats {
    cache_bytes_new: AtomicU64,
    cache_files_new: AtomicU64,
    cache_bytes_modified: AtomicU64,
    cache_files_modified: AtomicU64,
    cache_bytes_unchanged: AtomicU64,
    cache_files_unchanged: AtomicU64,

    bytes_copied: AtomicU64,
    files_copied: AtomicU64,
    bytes_skipped: AtomicU64,
    files_skipped: AtomicU64,
}

impl CopyStats {
    pub fn process(&self, result: &CopyResult) {
        match result.cache_result {
            FileCacheResult::New => {
                self.cache_file_new(result.metadata.file_size_bytes);
            }
            FileCacheResult::Modified => {
                self.cache_file_modified(result.metadata.file_size_bytes);
            }
            FileCacheResult::Unchanged => {
                self.cache_file_unchanged(result.metadata.file_size_bytes);
            }
        }
        match result.copy_result {
            FileCopyResult::Copied => {
                self.file_copied(result.metadata.file_size_bytes);
            }
            FileCopyResult::Skipped => {
                self.file_skipped(result.metadata.file_size_bytes);
            }
        }
    }

    fn cache_file_new(&self, file_size_bytes: u64) {
        self.cache_bytes_new
            .fetch_add(file_size_bytes, Ordering::Relaxed);
        self.cache_files_new.fetch_add(1, Ordering::Relaxed);
    }

    fn cache_file_modified(&self, file_size_bytes: u64) {
        self.cache_bytes_modified
            .fetch_add(file_size_bytes, Ordering::Relaxed);
        self.cache_files_modified.fetch_add(1, Ordering::Relaxed);
    }

    fn cache_file_unchanged(&self, file_size_bytes: u64) {
        self.cache_bytes_unchanged
            .fetch_add(file_size_bytes, Ordering::Relaxed);
        self.cache_files_unchanged.fetch_add(1, Ordering::Relaxed);
    }

    fn file_copied(&self, file_size_bytes: u64) {
        self.bytes_copied
            .fetch_add(file_size_bytes, Ordering::Relaxed);
        self.files_copied.fetch_add(1, Ordering::Relaxed);
    }

    fn file_skipped(&self, file_size_bytes: u64) {
        self.bytes_skipped
            .fetch_add(file_size_bytes, Ordering::Relaxed);
        self.files_skipped.fetch_add(1, Ordering::Relaxed);
    }

    pub fn get_stats(&self) -> StatsResult {
        StatsResult {
            total_bytes: self.bytes_copied.load(Ordering::Relaxed)
                + self.bytes_skipped.load(Ordering::Relaxed),
            total_files: self.files_copied.load(Ordering::Relaxed)
                + self.files_skipped.load(Ordering::Relaxed),
            cache_stats: CacheStats {
                new_bytes: self.cache_bytes_new.load(Ordering::Relaxed),
                new_files: self.cache_files_new.load(Ordering::Relaxed),
                modified_bytes: self.cache_bytes_modified.load(Ordering::Relaxed),
                modified_files: self.cache_files_modified.load(Ordering::Relaxed),
                unchanged_bytes: self.cache_bytes_unchanged.load(Ordering::Relaxed),
                unchanged_files: self.cache_files_unchanged.load(Ordering::Relaxed),
            },
            copy_stats: CopyStatus {
                copied_bytes: self.bytes_copied.load(Ordering::Relaxed),
                copied_files: self.files_copied.load(Ordering::Relaxed),
                skipped_bytes: self.bytes_skipped.load(Ordering::Relaxed),
                skipped_files: self.files_skipped.load(Ordering::Relaxed),
            },
        }
    }
}

pub struct StatsResult {
    pub total_bytes: u64,
    pub total_files: u64,
    pub cache_stats: CacheStats,
    pub copy_stats: CopyStatus,
}

pub struct CacheStats {
    pub new_bytes: u64,
    pub new_files: u64,
    pub modified_bytes: u64,
    pub modified_files: u64,
    pub unchanged_bytes: u64,
    pub unchanged_files: u64,
}

pub struct CopyStatus {
    pub copied_bytes: u64,
    pub copied_files: u64,
    pub skipped_bytes: u64,
    pub skipped_files: u64,
}
