use crate::progress::helpers::elapsed_subsec;
use indicatif::ProgressState;
use num_format::{Locale, ToFormattedString};
use std::fmt::Write;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use tracing::Span;
use tracing_indicatif::span_ext::IndicatifSpanExt;
use tracing_indicatif::style::ProgressStyle;

#[derive(Clone)]
pub struct CleanupProgress {
    shared: Arc<Shared>,
}

impl CleanupProgress {
    pub fn new(total: u64) -> (Self, Span) {
        let shared = Arc::new(Shared {
            total,
            ..Default::default()
        });

        let style = ProgressStyle::with_template(
            "{span_child_prefix}{spinner} Cleaning up, {remaining} remaining, {removed} removed {wide_msg:<} {elapsed_subsec}",
        )
        .unwrap()
        .with_key("elapsed_subsec", elapsed_subsec)
        .with_key("remaining", {
            let shared = shared.clone();
            move |_state: &ProgressState, writer: &mut dyn Write| {
                let _ = writer.write_str(&shared.get_remaining());
            }
        })
        .with_key("removed", {
            let shared = shared.clone();
            move |_state: &ProgressState, writer: &mut dyn Write| {
                let _ = writer.write_str(&shared.get_removed());
            }
        });

        let span = tracing::info_span!("Cleanup");
        span.pb_set_style(&style);

        (Self { shared }, span)
    }

    pub fn inc_checked(&self) {
        self.shared.checked.fetch_add(1, Ordering::Relaxed);
    }

    pub fn inc_removed(&self) {
        self.shared.removed.fetch_add(1, Ordering::Relaxed);
    }
}

#[derive(Default)]
struct Shared {
    total: u64,
    checked: AtomicU64,
    removed: AtomicU64,
}

impl Shared {
    pub fn get_remaining(&self) -> String {
        let checked = self.checked.load(Ordering::Relaxed);
        let remaining = self.total.saturating_sub(checked);
        remaining.to_formatted_string(&Locale::en)
    }

    pub fn get_removed(&self) -> String {
        let removed = self.removed.load(Ordering::Relaxed);
        removed.to_formatted_string(&Locale::en)
    }
}
