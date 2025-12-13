use thiserror::Error;

#[derive(Debug, Error)]
pub enum Error {
    #[error("Failed to read file: {0}")]
    Io(#[from] std::io::Error),

    #[error("Failed to parse binary: {0}")]
    ObjectParse(#[from] object::read::Error),

    #[error("No debug information found. Compile with -g flag to include DWARF debug info.")]
    NoDebugInfo,

    #[error("Unsupported binary format. Supported: ELF, Mach-O, PE.")]
    UnsupportedFormat,

    #[error("DWARF parsing error: {0}")]
    Dwarf(String),
}

pub type Result<T> = std::result::Result<T, Error>;
