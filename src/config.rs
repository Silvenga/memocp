use crate::cloning::CopyOp;
use bytesize::ByteSize;
use clap::Parser;
use std::{env, thread};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Config {
    /// The source directory/file to copy from.
    #[arg(index = 1, required = true)]
    pub source_path: String,

    /// The destination/file directory to copy to. If the directory does not exist, it will be created.
    #[arg(index = 2, required = true)]
    pub destination_path: String,

    /// The glob pattern to use for filtering files. Ignored if the source path is a file.
    /// Globs are matched case-insensitively.
    #[arg(long)]
    pub glob: Option<String>,

    /// The state file to use for memoization.
    #[arg(short, long, default_value_t = default_state_file())]
    pub state_file: String,

    /// Enable verbose logging.
    #[arg(short, long, default_value_t = false)]
    pub verbose: bool,

    /// The maximum number of threads to use for hashing and copying.
    /// An additional thread will always be used for scanning.
    /// Defaults to `8` or the number of CPU cores, whichever is smaller.
    #[arg(long, default_value_t = default_concurrency())]
    pub concurrency: usize,

    /// Take an exclusive lock on files during hashing.
    /// You likely only want to use this under Windows, where file locking is more reliable.
    #[arg(long)]
    pub exclusive_lock: bool,

    /// The number of bytes to read at a time when hashing files, per thread.
    /// Supports units like "KiB", "MiB", "GiB", etc.
    #[arg(long, default_value = "4 MiB")]
    pub hashing_read_chunk_size: ByteSize,

    /// Ignore hidden files.
    #[arg(long)]
    pub ignore_hidden: bool,

    /// Override existing files at the destination.
    #[arg(long = "override")]
    pub override_existing: bool,

    /// The copy mode to use.
    #[arg(long = "mode", value_enum, default_value_t = CopyOp::Reflink, ignore_case = true)]
    pub copy_mode: CopyOp,
}

fn default_concurrency() -> usize {
    let cores = thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1);
    cores.min(8)
}

fn default_state_file() -> String {
    env::current_dir()
        .expect("Working directory should always be available.")
        .join("memocp.db")
        .to_str()
        .expect("Current directory should always be valid UTF-8.")
        .to_owned()
}
