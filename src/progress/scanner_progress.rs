use crate::progress::helpers::elapsed_subsec;
use indicatif::ProgressState;
use num_format::{Locale, ToFormattedString};
use std::fmt::Write;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use tracing::Span;
use tracing_indicatif::span_ext::IndicatifSpanExt;
use tracing_indicatif::style::ProgressStyle;

#[derive(Clone)]
pub struct ScannerProgress {
    shared: Arc<Shared>,
}

impl ScannerProgress {
    pub fn new() -> (Self, Span) {
        let shared = Arc::new(Shared::default());

        let style = ProgressStyle::with_template(
            "{span_child_prefix}{spinner} Scanning, discovered {directories} directories and {files} files{blocked} {wide_msg:<} {elapsed_subsec}",
        )
        .unwrap()
        .with_key("elapsed_subsec", elapsed_subsec)
            .with_key("files", {
                let shared = shared.clone();
                move |_state: &ProgressState, writer: &mut dyn Write| {
                    let _ = writer.write_str(&shared.get_files());
                }
            })
            .with_key("directories", {
                let shared = shared.clone();
                move |_state: &ProgressState, writer: &mut dyn Write| {
                    let _ = writer.write_str(&shared.get_directories());
                }
            })
            .with_key("blocked", {
                let shared = shared.clone();
                move |_state: &ProgressState, writer: &mut dyn Write| {
                    let _ = writer.write_str(shared.get_blocked());
                }
            });

        let span = tracing::info_span!("");
        span.pb_set_style(&style);

        (Self { shared }, span)
    }

    pub fn inc_files(&self) {
        self.shared.files.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_directories(&self) {
        self.shared.directories.fetch_add(1, Ordering::Relaxed);
    }

    pub fn set_blocked(&self, blocked: bool) {
        self.shared.is_blocked.store(blocked, Ordering::Relaxed);
    }
}

#[derive(Default)]
struct Shared {
    files: AtomicU64,
    directories: AtomicU64,
    is_blocked: AtomicBool,
}

impl Shared {
    pub fn get_files(&self) -> String {
        let files = self.files.load(Ordering::Relaxed);
        files.to_formatted_string(&Locale::en)
    }

    pub fn get_directories(&self) -> String {
        let directories = self.directories.load(Ordering::Relaxed);
        directories.to_formatted_string(&Locale::en)
    }

    pub fn get_blocked(&self) -> &'static str {
        match self.is_blocked.load(Ordering::Relaxed) {
            true => " (Paused)",
            false => "",
        }
    }
}
