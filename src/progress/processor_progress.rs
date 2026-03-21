use crate::progress::helpers::elapsed_subsec;
use arc_swap::ArcSwap;
use indicatif::ProgressState;
use std::fmt::Write;
use std::path::Path;
use std::sync::Arc;
use tracing::Span;
use tracing_indicatif::span_ext::IndicatifSpanExt;
use tracing_indicatif::style::ProgressStyle;

pub struct ProcessorProgress {
    shared: Arc<Shared>,
}

impl ProcessorProgress {
    pub fn new(file: impl AsRef<Path>) -> (Self, Span) {
        let shared = Arc::new(Shared {
            stage: ArcSwap::new(ProcessorStage::Preparing.into()),
        });

        let style = ProgressStyle::with_template(
            "{span_child_prefix}{spinner} {stage} {wide_msg:<} {elapsed_subsec}",
        )
        .unwrap()
        .with_key("elapsed_subsec", elapsed_subsec)
        .with_key("stage", {
            let shared = shared.clone();
            move |_state: &ProgressState, writer: &mut dyn Write| {
                let _ = writer.write_str(shared.get_stage());
            }
        });

        let span = tracing::info_span!("");
        span.pb_set_style(&style);
        span.pb_set_message(&format!("{}", file.as_ref().display()));

        (Self { shared }, span)
    }

    pub fn set_stage(&self, stage: ProcessorStage) {
        self.shared.stage.store(stage.into());
    }
}

struct Shared {
    stage: ArcSwap<ProcessorStage>,
}

impl Shared {
    pub fn get_stage(&self) -> &'static str {
        let stage = self.stage.load();
        match stage.as_ref() {
            ProcessorStage::Preparing => "Preparing",
            ProcessorStage::Hashing => "Hashing",
            ProcessorStage::Copying => "Copying",
        }
    }
}

pub enum ProcessorStage {
    Preparing,
    Hashing,
    Copying,
}
