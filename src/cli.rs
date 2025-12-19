use clap::{Parser, Subcommand, ValueEnum};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "layout-audit")]
#[command(
    author,
    version,
    about = "Analyze binary memory layouts to detect padding inefficiencies"
)]
#[command(
    long_about = "layout-audit parses DWARF debugging information to visualize the physical \
layout of data structures, detect padding holes, and analyze cache line efficiency.\n\n\
Example:\n  layout-audit inspect ./target/debug/myapp --filter MyStruct"
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

        /// Output format (table, json, sarif)
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

        /// Warn about potential false sharing between atomic members on the same cache line
        #[arg(long)]
        warn_false_sharing: bool,

        /// Include Go runtime internal types (filtered by default)
        #[arg(long)]
        include_go_runtime: bool,
    },

    /// Compare struct layouts between two binaries
    Diff {
        /// Path to the old (baseline) binary
        #[arg(value_name = "OLD")]
        old: PathBuf,

        /// Path to the new binary
        #[arg(value_name = "NEW")]
        new: PathBuf,

        /// Filter structs by name (substring match)
        #[arg(short, long)]
        filter: Option<String>,

        /// Output format (table, json, sarif)
        #[arg(short, long, value_enum, default_value = "table")]
        output: OutputFormat,

        /// Cache line size in bytes (must be > 0)
        #[arg(long, default_value = "64", value_parser = clap::value_parser!(u32).range(1..))]
        cache_line: u32,

        /// Exit with error code 1 if any regressions found (size or padding increased)
        #[arg(long)]
        fail_on_regression: bool,

        /// Include Go runtime internal types (filtered by default)
        #[arg(long)]
        include_go_runtime: bool,
    },

    /// Check struct layouts against budget constraints
    Check {
        /// Path to the binary file to analyze
        #[arg(value_name = "BINARY")]
        binary: PathBuf,

        /// Path to config file (.layout-audit.yaml)
        #[arg(short, long, default_value = ".layout-audit.yaml")]
        config: PathBuf,

        /// Output format (table, json, sarif)
        #[arg(short, long, value_enum, default_value = "table")]
        output: OutputFormat,

        /// Cache line size in bytes (must be > 0)
        #[arg(long, default_value = "64", value_parser = clap::value_parser!(u32).range(1..))]
        cache_line: u32,

        /// Include Go runtime internal types (filtered by default)
        #[arg(long)]
        include_go_runtime: bool,
    },

    /// Suggest optimal field ordering to minimize padding
    Suggest {
        /// Path to the binary file to analyze
        #[arg(value_name = "BINARY")]
        binary: PathBuf,

        /// Filter structs by name (substring match)
        #[arg(short, long)]
        filter: Option<String>,

        /// Output format (table, json, sarif)
        #[arg(short, long, value_enum, default_value = "table")]
        output: OutputFormat,

        /// Show only structs with at least N bytes of potential savings
        #[arg(long)]
        min_savings: Option<u64>,

        /// Cache line size in bytes (must be > 0)
        #[arg(long, default_value = "64", value_parser = clap::value_parser!(u32).range(1..))]
        cache_line: u32,

        /// Pretty-print JSON output
        #[arg(long)]
        pretty: bool,

        /// Maximum alignment to assume for types (typically 8 on 64-bit)
        #[arg(long, default_value = "8", value_parser = clap::value_parser!(u64).range(1..))]
        max_align: u64,

        /// Sort suggestions by savings amount (largest first)
        #[arg(long)]
        sort_by_savings: bool,

        /// Disable colored output
        #[arg(long)]
        no_color: bool,

        /// Include Go runtime internal types (filtered by default)
        #[arg(long)]
        include_go_runtime: bool,
    },
}

#[derive(Copy, Clone, PartialEq, Eq, ValueEnum)]
pub enum OutputFormat {
    Table,
    Json,
    Sarif,
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
