use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Config {
    /// The source directory/file to copy from.
    #[arg(index = 1, required = true)]
    pub source_path: String,

    /// The destination/file directory to copy to.
    #[arg(index = 2, required = true)]
    pub destination_path: String,

    /// The state file to use for memoization.
    #[arg(short, long, default_value_t = String::from("./memocp.db"))]
    pub state_path: String,

    /// Enable verbose logging.
    #[arg(short, long, default_value_t = false)]
    pub verbose: bool,
}
