use std::io;
use thiserror::Error;
use tokio::task::JoinError;

#[derive(Error, Debug)]
pub enum HashingError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("Join error: {0}")]
    JoinError(#[from] JoinError),
    #[error("Failed to acquire exclusive on file: {0}")]
    FailedToLockFile(String),
}