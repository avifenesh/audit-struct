# CLI Architecture

## Component Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                      layout-audit CLI                           │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │                    Command Layer (clap)                   │  │
│  │  ┌─────────┐ ┌─────────┐ ┌─────────┐                     │  │
│  │  │ inspect │ │  diff   │ │  check  │                     │  │
│  │  └────┬────┘ └────┬────┘ └────┬────┘                     │  │
│  └───────┼───────────┼───────────┼──────────────────────────┘  │
│          │           │           │                              │
│          ▼           ▼           ▼                              │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │                   Analysis Engine                         │  │
│  │  • analyze_layout() - padding detection                   │  │
│  │  • diff_layouts() - binary comparison                     │  │
│  │  • Budget validation                                      │  │
│  └──────────────────────────────────────────────────────────┘  │
│                              │                                  │
│                              ▼                                  │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │                    DWARF Parser Layer                     │  │
│  │  • DwarfContext - gimli wrapper                           │  │
│  │  • TypeResolver - cached type resolution                  │  │
│  │  • Expression evaluation for member offsets               │  │
│  └──────────────────────────────────────────────────────────┘  │
│                              │                                  │
│                              ▼                                  │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │                   Object Loader Layer                     │  │
│  │  • ELF, Mach-O, PE support via `object` crate             │  │
│  │  • Memory-mapped I/O via `memmap2`                        │  │
│  │  • Compressed section handling                            │  │
│  └──────────────────────────────────────────────────────────┘  │
│                              │                                  │
│                              ▼                                  │
│  ┌──────────────────────────────────────────────────────────┐  │
│  │                    Output Formatters                      │  │
│  │  ┌──────────┐  ┌──────────┐                              │  │
│  │  │  Table   │  │   JSON   │                              │  │
│  │  └──────────┘  └──────────┘                              │  │
│  └──────────────────────────────────────────────────────────┘  │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

## Core Data Structures

```rust
pub struct StructLayout {
    pub name: String,
    pub size: u64,
    pub alignment: Option<u64>,
    pub source_location: Option<SourceLocation>,
    pub members: Vec<MemberLayout>,
    pub metrics: LayoutMetrics,
}

pub struct MemberLayout {
    pub name: String,
    pub type_name: String,
    pub offset: Option<u64>,
    pub size: Option<u64>,
    pub bit_offset: Option<u64>,  // For bitfields
    pub bit_size: Option<u64>,    // For bitfields
}

pub struct LayoutMetrics {
    pub total_size: u64,
    pub useful_size: u64,
    pub padding_bytes: u64,
    pub padding_percentage: f64,
    pub cache_lines_spanned: u32,
    pub cache_line_density: f64,
    pub padding_holes: Vec<PaddingHole>,
    pub partial: bool,  // True if some offsets were unknown
}

pub struct PaddingHole {
    pub offset: u64,
    pub size: u64,
    pub after_member: Option<String>,
}
```

## Analysis Pipeline

```
Binary File
    │
    ▼
┌─────────────────┐
│  Memory-map     │  memmap2::Mmap
│  the file       │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Parse object   │  object::File (ELF/Mach-O/PE)
│  format         │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Extract DWARF  │  Handle compressed sections
│  sections       │  (.zdebug_*)
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Create gimli   │  Dwarf<EndianSlice>
│  context        │
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Iterate CUs    │  Compilation Units
│  find structs   │  DW_TAG_structure_type
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Extract        │  Members, offsets, types
│  members        │  Bitfield handling
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Analyze        │  Padding detection
│  layout         │  Cache line analysis
└────────┬────────┘
         │
         ▼
┌─────────────────┐
│  Format         │  Table or JSON
│  output         │
└─────────────────┘
```

## Module Structure

```
src/
├── main.rs           # CLI entry, command dispatch
├── lib.rs            # Public API exports
├── cli.rs            # clap definitions
├── loader.rs         # Binary loading, DWARF extraction
├── types.rs          # Core data structures
├── error.rs          # Error types
├── diff.rs           # Layout comparison
├── dwarf/
│   ├── mod.rs
│   ├── context.rs    # DwarfContext, struct finder
│   ├── types.rs      # TypeResolver with caching
│   └── expr.rs       # DWARF expression evaluation
├── analysis/
│   ├── mod.rs
│   └── padding.rs    # Padding detection algorithm
└── output/
    ├── mod.rs
    ├── table.rs      # Terminal table formatter
    └── json.rs       # JSON formatter
```

## Dependencies

| Crate | Purpose |
|-------|---------|
| `gimli` | Zero-copy DWARF parsing |
| `object` | Binary format parsing (ELF/Mach-O/PE) |
| `memmap2` | Memory-mapped file I/O |
| `clap` | CLI argument parsing |
| `serde` | Serialization |
| `comfy-table` | Terminal tables |
| `colored` | Terminal colors |
