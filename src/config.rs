use crate::copying::CopyOp;
use bytesize::ByteSize;
use clap::Parser;
use std::{env, thread};

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Config {
    /// The source directory to copy from.
    #[arg(index = 1)]
    pub source_path: String,

    /// The destination directory to copy to. If the directory does not exist, it will be created.
    #[arg(index = 2, required_unless_present_any = ["load"])]
    pub destination_path: Option<String>,

    /// Scan the source directory to populate the database of "seen" file hashes without copying files.
    #[arg(long)]
    pub load: bool,

    /// Disable cleanup. This process prunes the cache for files that no longer exist.
    #[arg(long)]
    pub no_cleanup: bool,

    /// The glob pattern to use for filtering files. Ignored if the source path is a file.
    /// Globs are matched case-insensitively.
    #[arg(long)]
    pub glob: Option<String>,

    /// The state file to use for memoization.
    #[arg(short, long, default_value_t = default_state_file())]
    pub state_file: String,

    /// The maximum number of threads to use for file operations.
    /// An additional thread will always be used for scanning.
    /// Defaults to `8` or the number of CPU cores, whichever is smaller.
    #[arg(long, default_value_t = default_concurrency())]
    pub concurrency: usize,

    /// The maximum size of the discovery queue before the scanner will pause.
    #[arg(long, default_value_t = 100_000)]
    pub queue_depth: usize,

    /// Take an exclusive lock on files during hashing.
    /// You likely only want to use this under Windows, where file locking is more reliable.
    #[arg(long)]
    pub exclusive_lock: bool,

    /// The number of bytes to read at a time when hashing files, per thread.
    /// Supports units like "KiB", "MiB", "GiB", etc.
    #[arg(long, default_value = "4 MiB")]
    pub hashing_read_chunk_size: ByteSize,

    /// Override existing files at the destination.
    #[arg(long = "override")]
    pub override_existing: bool,

    /// The copy mode to use.
    #[arg(long = "mode", value_enum, default_value_t = CopyOp::Reflink, ignore_case = true)]
    pub copy_mode: CopyOp,

    /// Ignore hidden files.
    #[arg(long)]
    pub ignore_hidden: bool,

    /// Enable verbose logging.
    #[arg(short, long, default_value_t = false)]
    pub verbose: bool,
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
