use crate::config::Config;
use crate::runner::Runner;
use clap::Parser;
use std::error::Error;
use tracing::log::LevelFilter;

mod cli;
mod config;
mod db;
mod runner;
mod scanning;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    log_panics::init();

    let config = Config::parse();

    env_logger::builder()
        .filter_level(if config.verbose {
            LevelFilter::Trace
        } else {
            LevelFilter::Info
        })
        .init();

    let runner = Runner::new(config).await?;
    runner.run().await;

    Ok(())
}
