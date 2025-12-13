pub mod analysis;
pub mod cli;
pub mod diff;
pub mod dwarf;
pub mod error;
pub mod loader;
pub mod output;
pub mod types;

pub use analysis::{analyze_false_sharing, analyze_layout};
pub use cli::{Cli, Commands, OutputFormat, SortField};
pub use diff::{DiffResult, diff_layouts};
pub use dwarf::DwarfContext;
pub use error::{Error, Result};
pub use loader::{BinaryData, LoadedDwarf};
pub use output::{JsonFormatter, TableFormatter};
pub use types::{
    AtomicMember, CacheLineSpanningWarning, FalseSharingAnalysis, FalseSharingWarning,
    LayoutMetrics, MemberLayout, PaddingHole, SourceLocation, StructLayout,
};
