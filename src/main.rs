use crate::config::Config;
use crate::runner::Runner;
use clap::Parser;
use std::error::Error;
use tracing::level_filters::LevelFilter;
use tracing_indicatif::IndicatifLayer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

mod cleanup;
mod config;
mod copying;
mod db;
mod hashing;
mod models;
mod progress;
mod runner;
mod scanner;
mod stats;
mod templating;
mod worker;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let config = Config::parse();

    let log_level = if config.verbose {
        LevelFilter::TRACE
    } else {
        LevelFilter::INFO
    };

    let indicatif_layer = IndicatifLayer::new().with_max_progress_bars(
        config
            .concurrency
            .saturating_mul(2)
            .try_into()
            .unwrap_or(24),
        None,
    );

    tracing_subscriber::registry()
        .with(log_level)
        .with(
            tracing_subscriber::fmt::layer()
                .with_writer(indicatif_layer.get_stderr_writer())
                .with_target(false),
        )
        .with(indicatif_layer)
        .init();

    log_panics::init();

    let runner = Runner::new(config).await?;

    tokio::select! {
        result = runner.run() => {
            result?;
        }
        _ = tokio::signal::ctrl_c() => {
            eprintln!("Ctrl+C received, finishing outstanding tasks...");
        }
    }

    Ok(())
}
