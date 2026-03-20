use indicatif::ProgressState;
use std::fmt::Write;

pub fn elapsed_subsec(state: &ProgressState, writer: &mut dyn Write) {
    let seconds = state.elapsed().as_secs();
    let sub_seconds = (state.elapsed().as_millis() % 1000) / 100;
    let _ = writer.write_str(&format!("{}.{}s", seconds, sub_seconds));
}
