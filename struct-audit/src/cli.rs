use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "struct-audit")]
#[command(
    author,
    version,
    about = "Analyze binary memory layouts to detect padding inefficiencies"
)]
#[command(
    long_about = "struct-audit parses DWARF debugging information to visualize the physical \
layout of data structures, detect padding holes, and analyze cache line efficiency.\n\n\
Example:\n  struct-audit inspect ./target/debug/myapp --filter MyStruct"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Analyze and display struct layouts from a binary
    Inspect {
        /// Path to the binary file to analyze
        #[arg(value_name = "BINARY")]
        binary: PathBuf,

        /// Filter structs by name (substring match)
        #[arg(short, long)]
        filter: Option<String>,

        /// Output format
        #[arg(short, long, value_enum, default_value = "table")]
        output: OutputFormat,

        /// Disable colored output
        #[arg(long)]
        no_color: bool,

        /// Cache line size in bytes
        #[arg(long, default_value = "64")]
        cache_line: u32,

        /// Pretty-print JSON output
        #[arg(long)]
        pretty: bool,
    },
}

#[derive(Copy, Clone, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    Table,
    Json,
}
