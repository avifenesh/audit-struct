pub mod analysis;
pub mod cli;
pub mod diff;
pub mod dwarf;
pub mod error;
pub mod loader;
pub mod output;
pub mod types;

pub use analysis::{
    OptimizedLayout, OptimizedMember, analyze_false_sharing, analyze_layout, optimize_layout,
};
pub use cli::{Cli, Commands, OutputFormat, SortField};
pub use diff::{DiffResult, diff_layouts};
pub use dwarf::DwarfContext;
pub use error::{Error, Result};
pub use loader::{BinaryData, LoadedDwarf};
pub use output::{JsonFormatter, SuggestJsonFormatter, SuggestTableFormatter, TableFormatter};
pub use types::{
    AtomicMember, CacheLineSpanningWarning, FalseSharingAnalysis, FalseSharingWarning,
    LayoutMetrics, MemberLayout, PaddingHole, SourceLocation, StructLayout,
};
