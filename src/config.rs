use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Config {
    /// The source directory to copy files from.
    #[arg(index = 1, required = true)]
    pub source_directory: String,

    /// The destination directory to copy files to.
    #[arg(index = 2, required = true)]
    pub destination_directory: String,

    /// The state file to use for memoization.
    #[arg(short, long, default_value_t = String::from("./memocp.db"))]
    pub state: String,

    /// Enable verbose logging.
    #[arg(short, long, default_value_t = false)]
    pub verbose: bool,
}
