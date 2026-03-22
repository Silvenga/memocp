use super::{Hash, HashingError};
use crate::task::spawn_blocking_with_cancellation;
use blake3::Hasher as Blake3Hasher;
use fs4::fs_std::FileExt;
use std::fs::File;
use std::io;
use std::io::Read;
use std::path::{Path, PathBuf};
use tokio_util::sync::CancellationToken;

pub struct Hasher {
    read_chunk_size: u64,
    take_exclusive_lock: bool,
}

impl Default for Hasher {
    fn default() -> Self {
        Self {
            read_chunk_size: 4 * 1024 * 1024, // 4 MiB
            take_exclusive_lock: false,
        }
    }
}

impl Hasher {
    pub fn with_read_chunk_size(mut self, read_chunk_size: u64) -> Self {
        self.read_chunk_size = read_chunk_size;
        self
    }

    pub fn with_take_exclusive_lock(mut self, take_exclusive_lock: bool) -> Self {
        self.take_exclusive_lock = take_exclusive_lock;
        self
    }

    pub async fn hash_file(&self, file_path: impl AsRef<Path>) -> Result<Hash, HashingError> {
        spawn_blocking_with_cancellation({
            let read_chunk_size = self.read_chunk_size;
            let take_exclusive_lock = self.take_exclusive_lock;
            let file_path = file_path.as_ref().to_owned();
            move |cancellation_token| -> Result<Hash, HashingError> {
                let _span = tracing::trace_span!("Hashing file").entered();
                let file = {
                    if take_exclusive_lock {
                        Self::open_file_exclusively(file_path)?
                    } else {
                        File::open(file_path)?
                    }
                };
                let hash = Self::hash_file_stream(file, read_chunk_size, cancellation_token)?;
                Ok(hash)
            }
        })
        .await?
    }

    fn open_file_exclusively(file_path: PathBuf) -> Result<File, HashingError> {
        let file = File::open(file_path.clone())?;

        if !file.try_lock_exclusive()? {
            return Err(HashingError::FailedToLockFile(
                file_path.to_string_lossy().to_string(),
            ));
        }
        Ok(file)
    }

    fn hash_file_stream(
        mut file: File,
        read_chunk_size: u64,
        cancellation_token: CancellationToken,
    ) -> io::Result<Hash> {
        let mut hasher = Blake3Hasher::new();
        let mut buffer = vec![0; read_chunk_size.try_into().unwrap_or(usize::MAX)];

        loop {
            match file.read(&mut buffer) {
                Ok(0) => break, // EOF
                Ok(n) => {
                    hasher.update(&buffer[..n]);
                }
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(e),
            }
            if cancellation_token.is_cancelled() {
                tracing::trace!("Hashing was cancelled.");
                return Err(io::Error::new(
                    io::ErrorKind::Interrupted,
                    "Operation cancelled",
                ));
            }
        }

        Ok(hasher.finalize().into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assert_fs::TempDir;
    use assert_fs::prelude::*;
    use assert_matches::assert_matches;

    #[tokio::test]
    async fn can_hash_file() {
        let temp = TempDir::new().unwrap();
        let file = temp.child("foo.txt");
        file.write_str("hello world").unwrap();

        let hash = Hasher::default()
            .hash_file(file.to_path_buf())
            .await
            .unwrap();

        assert_eq!(
            "d74981efa70a0c880b8d8c1985d075dbcbf679b99a5f9914e5aaf96b831a9e24",
            hash.as_string()
        );
    }

    #[tokio::test]
    async fn when_file_is_locked_then_hash_file_should_error() {
        let temp = TempDir::new().unwrap();
        let file = temp.child("foo.txt");
        file.write_str("hello world").unwrap();

        let _locked_file = Hasher::open_file_exclusively(file.to_path_buf()).unwrap();

        let result = Hasher::default()
            .with_take_exclusive_lock(true)
            .hash_file(file.to_path_buf())
            .await;

        assert_matches!(result, Err(HashingError::FailedToLockFile(_)))
    }
}
