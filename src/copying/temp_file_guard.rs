use std::fs;
use std::path::{Path, PathBuf};

pub struct TempFileGuard {
    file: Option<PathBuf>,
}

impl TempFileGuard {
    pub fn new(path: impl AsRef<Path>) -> Self {
        Self {
            file: Some(path.as_ref().to_path_buf()),
        }
    }

    pub fn disarm(&mut self) {
        self.file = None;
    }
}

impl Drop for TempFileGuard {
    fn drop(&mut self) {
        if let Some(path) = &self.file {
            tracing::debug!("Removing temporary file {:?}.", path);
            let _ = fs::remove_file(path);
        }
    }
}
