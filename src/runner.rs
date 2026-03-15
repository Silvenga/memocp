use crate::cloning::{Copier, CopyStats};
use crate::config::Config;
use crate::db::Db;
use crate::hashing::Hasher;
use crate::scanner::Scanner;
use crate::templating::Templater;
use bytesize::ByteSize;
use futures::StreamExt;
use humantime::format_duration;
use std::error::Error;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Instant;
use tokio::io;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tracing::Instrument;

const MAX_SCANNING_QUEUE_SIZE: usize = 100_000;

pub struct Runner {
    config: Config,
    db: Db,
}

impl Runner {
    pub async fn new(config: Config) -> Result<Self, Box<dyn Error>> {
        let db = Db::open_file(config.state_file.clone()).await?;
        Ok(Self { config, db })
    }

    pub async fn run(&self) -> anyhow::Result<()> {
        let source_path = self.get_source_path()?;

        let scanner = Scanner::new(&source_path)
            .with_globs(self.config.glob.iter().cloned().collect())
            .with_ignore_hidden(self.config.ignore_hidden);

        let hasher = Hasher::default()
            .with_take_exclusive_lock(self.config.exclusive_lock)
            .with_read_chunk_size(self.config.hashing_read_chunk_size.as_u64());

        let templater = Templater::new(&source_path, &self.config.destination_path);

        let copier = Copier::new(self.db.clone(), hasher, templater)
            .with_copy_op(self.config.copy_mode)
            .with_override_existing(self.config.override_existing);

        let copy_stats = Arc::from(CopyStats::default());

        let start_time = Instant::now();

        let (tx, rx) = mpsc::channel::<PathBuf>(MAX_SCANNING_QUEUE_SIZE);
        let scanner_task = scanner.scan(tx);
        let processing_tasks =
            ReceiverStream::new(rx).for_each_concurrent(self.config.concurrency, {
                let copier = Arc::from(copier);
                let copy_stats = copy_stats.clone();
                move |file| {
                    let copier = copier.clone();
                    let copy_stats = copy_stats.clone();
                    let span = tracing::info_span!(
                        "Processing file",
                        file = file.to_string_lossy().to_string()
                    );
                    async move {
                        match copier.copy(&file).await {
                            Ok(result) => {
                                copy_stats.process(&result);
                            }
                            Err(e) => {
                                tracing::warn!(
                                    "Failed to copy file {}: {}",
                                    file.to_string_lossy(),
                                    e
                                );
                            }
                        };
                    }
                    .instrument(span)
                }
            });

        let (scanner_result, _) = tokio::join!(scanner_task, processing_tasks);

        if scanner_result.is_ok() {
            let stats = copy_stats.get_stats();
            tracing::info!(
                "Processed {} files ({}) in {}.",
                stats.total_files,
                ByteSize::b(stats.total_bytes),
                format_duration(start_time.elapsed())
            );
            tracing::info!(
                "Hashing: {} ({}) new, {} ({}) modified, {} ({}) unchanged.",
                stats.cache_stats.new_files,
                ByteSize::b(stats.cache_stats.new_bytes),
                stats.cache_stats.modified_files,
                ByteSize::b(stats.cache_stats.modified_bytes),
                stats.cache_stats.unchanged_files,
                ByteSize::b(stats.cache_stats.unchanged_bytes),
            );
            tracing::info!(
                "Copying: {} ({}) copied, {} ({}) skipped.",
                stats.copy_stats.copied_files,
                ByteSize::b(stats.copy_stats.copied_bytes),
                stats.copy_stats.skipped_files,
                ByteSize::b(stats.copy_stats.skipped_bytes),
            );
        }

        scanner_result
    }

    fn get_source_path(&self) -> io::Result<PathBuf> {
        dunce::canonicalize(&self.config.source_path)
    }
}
