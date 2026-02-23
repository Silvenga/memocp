use super::{Hash, HashingError};
use blake3::Hasher as Blake3Hasher;
use fs4::fs_std::FileExt;
use std::fs::File;
use std::io;
use std::path::PathBuf;
use tokio::task;

#[derive(Default)]
pub struct Hasher;

impl Hasher {
    pub async fn hash_file(file_path: PathBuf) -> Result<Hash, HashingError> {
        task::spawn_blocking(move || -> Result<Hash, HashingError> {
            let file = Self::open_file_exclusively(file_path)?;
            let hash = Self::hash_file_via_nmap(file)?;
            Ok(hash)
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

    fn hash_file_via_nmap(file: File) -> io::Result<Hash> {
        let mut hasher = Blake3Hasher::new();
        if let Some(mmap) = Self::maybe_mmap_file(&file)? {
            hasher.update(&mmap);
        } else {
            Self::copy_wide(&file, &mut hasher)?;
        }
        let hash = hasher.finalize().into();
        Ok(hash)
    }

    /// From blake3.
    fn maybe_mmap_file(file: &File) -> io::Result<Option<memmap2::Mmap>> {
        let metadata = file.metadata()?;
        let file_size = metadata.len();
        if !metadata.is_file() {
            // Not a real file.
            Ok(None)
        } else if file_size < 16 * 1024 {
            // Mapping small files is not worth it, and some special files that can't be mapped report
            // a size of zero.
            Ok(None)
        } else {
            let map = unsafe { memmap2::Mmap::map(file)? };
            Ok(Some(map))
        }
    }

    /// From blake3.
    fn copy_wide(mut reader: impl io::Read, hasher: &mut Blake3Hasher) -> io::Result<u64> {
        let mut buffer = [0; 65536];
        let mut total = 0;
        loop {
            match reader.read(&mut buffer) {
                Ok(0) => return Ok(total),
                Ok(n) => {
                    hasher.update(&buffer[..n]);
                    total += n as u64;
                }
                // see test_update_reader_interrupted
                Err(e) if e.kind() == io::ErrorKind::Interrupted => continue,
                Err(e) => return Err(e),
            }
        }
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

        let hash = Hasher::hash_file(file.to_path_buf()).await.unwrap();

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

        let result = Hasher::hash_file(file.to_path_buf()).await;

        assert_matches!(result, Err(HashingError::FailedToLockFile(_)))
    }
}
