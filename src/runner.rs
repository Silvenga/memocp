use crate::config::Config;
use crate::copier::Copier;
use crate::db::Db;
use crate::hashing::Hasher;
use crate::scanner::Scanner;
use futures::StreamExt;
use std::error::Error;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::io;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tracing::Instrument;
use crate::templating::Templater;

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
        let copier = Arc::from(Copier::new(self.db.clone(), hasher, templater));

        let (tx, rx) = mpsc::channel::<PathBuf>(MAX_SCANNING_QUEUE_SIZE);
        let scanner_task = scanner.scan(tx);
        let processing_tasks =
            ReceiverStream::new(rx).for_each_concurrent(self.config.concurrency, {
                let copier = copier.clone();
                move |file| {
                    let copier = copier.clone();
                    let span = tracing::info_span!(
                        "Processing file",
                        file = file.to_string_lossy().to_string()
                    );
                    async move {
                        match copier.copy(&file).await {
                            Ok(_) => {}
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
        scanner_result
    }

    fn get_source_path(&self) -> io::Result<PathBuf> {
        dunce::canonicalize(&self.config.source_path)
    }
}
