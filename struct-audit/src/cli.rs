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

        /// Sort structs by field
        #[arg(short, long, value_enum, default_value = "name")]
        sort_by: SortField,

        /// Show only the top N structs (by sort order)
        #[arg(short = 'n', long)]
        top: Option<usize>,

        /// Show only structs with at least N bytes of padding
        #[arg(long)]
        min_padding: Option<u64>,

        /// Disable colored output
        #[arg(long)]
        no_color: bool,

        /// Cache line size in bytes (must be > 0)
        #[arg(long, default_value = "64", value_parser = clap::value_parser!(u32).range(1..))]
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

#[derive(Copy, Clone, PartialEq, Eq, ValueEnum)]
pub enum SortField {
    /// Sort by struct name (alphabetical)
    Name,
    /// Sort by total size (largest first)
    Size,
    /// Sort by padding bytes (most padding first)
    Padding,
    /// Sort by padding percentage (worst efficiency first)
    PaddingPct,
}
