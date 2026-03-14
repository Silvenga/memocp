use ignore::WalkBuilder;
use ignore::overrides::OverrideBuilder;
use std::path::{Path, PathBuf};
use tokio::sync::mpsc;
use tokio::task;

pub struct Scanner {
    source_path: PathBuf,
    globs: Vec<String>,
    ignore_hidden: bool,
}

impl Scanner {
    pub fn new(source_path: impl AsRef<Path>) -> Self {
        Self {
            source_path: source_path.as_ref().to_owned(),
            globs: Vec::default(),
            ignore_hidden: false,
        }
    }

    pub fn with_globs(mut self, globs: Vec<String>) -> Self {
        self.globs = globs;
        self
    }

    pub fn with_ignore_hidden(mut self, ignore_hidden: bool) -> Self {
        self.ignore_hidden = ignore_hidden;
        self
    }

    pub async fn scan(&self, tx: mpsc::Sender<PathBuf>) -> anyhow::Result<()> {
        tracing::debug!(
            "Source scanning started in {:?}, hidden files ignored: {}.",
            self.source_path,
            self.ignore_hidden
        );

        let walker = {
            let mut builder = WalkBuilder::new(&self.source_path);
            builder.sort_by_file_path(|a, b| a.cmp(b));
            builder.hidden(self.ignore_hidden);

            let overrides = {
                let mut builder = OverrideBuilder::new(&self.source_path);
                builder.case_insensitive(true)?;
                for glob in &self.globs {
                    builder.add(glob)?;
                }
                builder.build()
            }?;
            builder.overrides(overrides);

            builder.build()
        };

        task::spawn_blocking({
            move || {
                let _span = tracing::info_span!("Scanning source paths").entered();

                let mut total = 0u64;
                for result in walker {
                    match result {
                        Ok(entry) => {
                            if let Some(file_type) = entry.file_type()
                                && file_type.is_file()
                                && tx.blocking_send(entry.into_path()).is_err()
                            {
                                break;
                            }
                            total += 1;
                        }
                        Err(error) => {
                            tracing::warn!("Encountered non-fatal error while scanning: {error}.");
                        }
                    }
                }

                tracing::info!("Source scanning complete, discovered {total} files.");
            }
        })
        .await?;

        Ok(())
    }
}
