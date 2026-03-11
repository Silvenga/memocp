use ignore::WalkBuilder;
use std::path::PathBuf;
use tokio::sync::mpsc;
use tokio::task;

#[derive(Default)]
pub struct Scanner {
    source_paths: Vec<String>,
    ignore_hidden: bool,
}

impl Scanner {
    pub fn with_source_paths(mut self, source_paths: &[String]) -> Self {
        self.source_paths = source_paths.to_vec();
        self
    }
    pub fn with_ignore_hidden(mut self, ignore_hidden: bool) -> Self {
        self.ignore_hidden = ignore_hidden;
        self
    }

    pub async fn scan(&self, tx: mpsc::Sender<PathBuf>) {
        let [first_source, other_sources @ ..] = self.source_paths.as_slice() else {
            return;
        };
        task::spawn_blocking({
            let first_source = first_source.clone();
            let other_sources = other_sources.to_vec();
            let ignore_hidden = self.ignore_hidden;
            move || {
                let _span = tracing::info_span!("Scanning source paths").entered();

                let walker = {
                    let mut builder = WalkBuilder::new(&first_source);
                    builder.sort_by_file_path(|a, b| a.cmp(b));
                    builder.hidden(ignore_hidden);
                    for source in &other_sources {
                        builder.add(source);
                    }
                    builder.build()
                };
                tracing::debug!("Source scanning started in {first_source} and [{other_sources:?}], hidden files ignored: {ignore_hidden}.");

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
        .await
        .unwrap();
    }
}
