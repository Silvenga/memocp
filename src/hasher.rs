use blake3::Hasher;
use fs4::fs_std::FileExt;
use std::fs::File;
use std::io;
use std::path::PathBuf;
use thiserror::Error;
use tokio::task;
use tokio::task::JoinError;

pub type Hash = blake3::Hash;

pub async fn hash_file(file_path: PathBuf) -> Result<Hash, Error> {
    task::spawn_blocking(move || -> Result<Hash, Error> {
        let file = open_file_exclusively(file_path)?;
        let hash = hash_file_via_nmap(file)?;
        Ok(hash)
    })
    .await?
}

fn open_file_exclusively(file_path: PathBuf) -> Result<File, Error> {
    let file = File::open(file_path.clone())?;
    if !file.try_lock_exclusive()? {
        return Err(Error::FailedToLockFile(
            file_path.to_string_lossy().to_string(),
        ));
    }
    Ok(file)
}

fn hash_file_via_nmap(file: File) -> io::Result<Hash> {
    let mut hasher = Hasher::new();
    if let Some(mmap) = maybe_mmap_file(&file)? {
        hasher.update(&mmap);
    } else {
        copy_wide(&file, &mut hasher)?;
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
fn copy_wide(mut reader: impl io::Read, hasher: &mut Hasher) -> io::Result<u64> {
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

#[derive(Error, Debug)]
pub enum Error {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("Join error: {0}")]
    JoinError(#[from] JoinError),
    #[error("Failed to acquire exclusive on file: {0}")]
    FailedToLockFile(String),
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

        let hash = hash_file(file.to_path_buf()).await.unwrap();

        assert_eq!(
            "d74981efa70a0c880b8d8c1985d075dbcbf679b99a5f9914e5aaf96b831a9e24",
            hash.to_hex().to_string()
        );
    }

    #[tokio::test]
    async fn when_file_is_locked_then_hash_file_should_error() {
        let temp = TempDir::new().unwrap();
        let file = temp.child("foo.txt");
        file.write_str("hello world").unwrap();

        let _locked_file = open_file_exclusively(file.to_path_buf()).unwrap();

        let result = hash_file(file.to_path_buf()).await;

        assert_matches!(result, Err(Error::FailedToLockFile(_)))
    }
}
