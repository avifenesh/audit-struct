pub mod analysis;
pub mod cli;
pub mod dwarf;
pub mod error;
pub mod loader;
pub mod output;
pub mod types;

pub use analysis::analyze_layout;
pub use cli::{Cli, Commands, OutputFormat, SortField};
pub use dwarf::DwarfContext;
pub use error::{Error, Result};
pub use loader::{BinaryData, LoadedDwarf};
pub use output::{JsonFormatter, TableFormatter};
pub use types::{LayoutMetrics, MemberLayout, PaddingHole, SourceLocation, StructLayout};
