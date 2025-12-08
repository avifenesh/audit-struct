# Detailed Task Breakdown

## Executive Summary

This document provides granular, actionable tasks for implementing struct-audit. Each task is estimated, has clear acceptance criteria, and identifies dependencies.

---

## Task Estimation Guide

| Size | Description | Typical Duration |
|------|-------------|------------------|
| XS | Trivial change, single file | 1-2 hours |
| S | Small feature, few files | 0.5-1 day |
| M | Medium feature, multiple components | 1-2 days |
| L | Large feature, significant scope | 3-5 days |
| XL | Epic, should be broken down further | 1+ week |

---

## Phase 1: Core CLI Development

### Epic 1.1: Project Setup

#### Task 1.1.1: Initialize Rust Workspace
**Size**: XS | **Priority**: P0 | **Dependencies**: None

**Description**: Create Cargo workspace with proper structure.

**Acceptance Criteria**:
- [ ] `cargo new struct-audit --lib` for core library
- [ ] `cargo new struct-audit-cli --bin` for CLI binary
- [ ] Workspace Cargo.toml configured
- [ ] CI workflow (GitHub Actions) with `cargo check`, `cargo test`, `cargo clippy`
- [ ] `.gitignore`, `LICENSE` (MIT + Apache 2.0), basic `README.md`

**Files**:
```
struct-audit/
├── Cargo.toml (workspace)
├── struct-audit-core/
│   ├── Cargo.toml
│   └── src/lib.rs
├── struct-audit-cli/
│   ├── Cargo.toml
│   └── src/main.rs
├── .github/workflows/ci.yml
└── README.md
```

---

#### Task 1.1.2: Add Core Dependencies
**Size**: XS | **Priority**: P0 | **Dependencies**: 1.1.1

**Description**: Add and configure essential crates.

**Acceptance Criteria**:
- [ ] `gimli = "0.28"` for DWARF parsing
- [ ] `object = "0.32"` for binary format handling
- [ ] `memmap2 = "0.9"` for memory-mapped file I/O
- [ ] `serde = { version = "1.0", features = ["derive"] }` for serialization
- [ ] `serde_json = "1.0"` for JSON output
- [ ] `clap = { version = "4.4", features = ["derive"] }` for CLI
- [ ] `anyhow = "1.0"` and `thiserror = "1.0"` for error handling
- [ ] `comfy-table = "7.0"` for text tables

**Cargo.toml** (core):
```toml
[dependencies]
gimli = "0.28"
object = "0.32"
memmap2 = "0.9"
serde = { version = "1.0", features = ["derive"] }
thiserror = "1.0"
```

---

### Epic 1.2: Binary Loading

#### Task 1.2.1: Implement Binary Loader Trait
**Size**: S | **Priority**: P0 | **Dependencies**: 1.1.2

**Description**: Create abstraction for loading different binary formats.

**Acceptance Criteria**:
- [ ] `BinaryLoader` trait with `load()` method
- [ ] Returns `DwarfSections` struct with all required sections
- [ ] Error types for missing debug info, unsupported format

**Code Sketch**:
```rust
pub struct DwarfSections<'data> {
    pub debug_info: &'data [u8],
    pub debug_abbrev: &'data [u8],
    pub debug_str: &'data [u8],
    pub debug_line: Option<&'data [u8]>,
    pub debug_ranges: Option<&'data [u8]>,
}

pub trait BinaryLoader {
    fn load(&self, path: &Path) -> Result<DwarfSections<'_>>;
    fn endianness(&self) -> Endianness;
    fn pointer_size(&self) -> usize;
}
```

---

#### Task 1.2.2: Implement ELF Loader
**Size**: M | **Priority**: P0 | **Dependencies**: 1.2.1

**Description**: Load DWARF sections from ELF binaries.

**Acceptance Criteria**:
- [ ] Parse ELF header and section table
- [ ] Extract `.debug_*` sections
- [ ] Handle compressed sections (`.zdebug_*`, `SHF_COMPRESSED`)
- [ ] Support both 32-bit and 64-bit ELF
- [ ] Unit tests with sample ELF files

---

#### Task 1.2.3: Implement Mach-O Loader
**Size**: M | **Priority**: P1 | **Dependencies**: 1.2.1

**Description**: Load DWARF sections from Mach-O binaries (macOS).

**Acceptance Criteria**:
- [ ] Parse Mach-O header and load commands
- [ ] Extract `__DWARF` segment sections
- [ ] Handle universal (fat) binaries
- [ ] Unit tests with sample Mach-O files

---

#### Task 1.2.4: Implement PE Loader
**Size**: M | **Priority**: P2 | **Dependencies**: 1.2.1

**Description**: Load DWARF sections from PE/COFF binaries (Windows).

**Acceptance Criteria**:
- [ ] Parse PE headers
- [ ] Extract `.debug_*` sections
- [ ] Note: Full PDB support is out of scope for MVP
- [ ] Unit tests with sample PE files

---

### Epic 1.3: DWARF Parsing Core

#### Task 1.3.1: Initialize gimli Context
**Size**: S | **Priority**: P0 | **Dependencies**: 1.2.2

**Description**: Set up gimli::Dwarf from loaded sections.

**Acceptance Criteria**:
- [ ] Create `gimli::Dwarf` from `DwarfSections`
- [ ] Configure endianness correctly
- [ ] Handle missing optional sections gracefully

**Code Sketch**:
```rust
pub fn create_dwarf_context<'data>(
    sections: DwarfSections<'data>,
    endian: gimli::RunTimeEndian,
) -> Result<gimli::Dwarf<gimli::EndianSlice<'data, gimli::RunTimeEndian>>> {
    // Implementation
}
```

---

#### Task 1.3.2: Implement Compilation Unit Iterator
**Size**: S | **Priority**: P0 | **Dependencies**: 1.3.1

**Description**: Iterate over all compilation units in DWARF data.

**Acceptance Criteria**:
- [ ] Iterate `dwarf.units()`
- [ ] Parse CU headers
- [ ] Extract DWARF version per CU
- [ ] Skip units without type information

---

#### Task 1.3.3: Implement DIE Traversal
**Size**: M | **Priority**: P0 | **Dependencies**: 1.3.2

**Description**: Traverse DIE tree to find struct definitions.

**Acceptance Criteria**:
- [ ] Depth-first traversal of DIEs
- [ ] Filter for `DW_TAG_structure_type`, `DW_TAG_class_type`, `DW_TAG_union_type`
- [ ] Collect child `DW_TAG_member` entries
- [ ] Handle `DW_TAG_inheritance` for C++ classes

---

#### Task 1.3.4: Implement Attribute Extraction
**Size**: M | **Priority**: P0 | **Dependencies**: 1.3.3

**Description**: Extract relevant attributes from DIEs.

**Acceptance Criteria**:
- [ ] Extract `DW_AT_name` (resolve from string table)
- [ ] Extract `DW_AT_byte_size`
- [ ] Extract `DW_AT_data_member_location`
- [ ] Extract `DW_AT_type` references
- [ ] Handle missing attributes gracefully

---

#### Task 1.3.5: Implement Simple Location Resolution
**Size**: S | **Priority**: P0 | **Dependencies**: 1.3.4

**Description**: Handle constant-value `DW_AT_data_member_location`.

**Acceptance Criteria**:
- [ ] Parse `DW_FORM_data*` location values
- [ ] Parse `DW_FORM_udata` location values
- [ ] Return byte offset as integer

---

### Epic 1.4: Type Resolution

#### Task 1.4.1: Implement Type Reference Resolver
**Size**: M | **Priority**: P0 | **Dependencies**: 1.3.4

**Description**: Follow `DW_AT_type` references to determine sizes.

**Acceptance Criteria**:
- [ ] Resolve `DW_TAG_base_type` → get `DW_AT_byte_size`
- [ ] Resolve `DW_TAG_pointer_type` → use architecture pointer size
- [ ] Resolve `DW_TAG_typedef` → follow to underlying type
- [ ] Resolve `DW_TAG_const_type`, `DW_TAG_volatile_type` → follow chain
- [ ] Cache resolved types to avoid repeated traversal

---

#### Task 1.4.2: Implement Array Type Handling
**Size**: M | **Priority**: P0 | **Dependencies**: 1.4.1

**Description**: Calculate size of array types.

**Acceptance Criteria**:
- [ ] Parse `DW_TAG_array_type`
- [ ] Find `DW_TAG_subrange_type` child for bounds
- [ ] Calculate: element_size × (upper_bound - lower_bound + 1)
- [ ] Handle multi-dimensional arrays

---

#### Task 1.4.3: Implement Nested Struct Resolution
**Size**: M | **Priority**: P1 | **Dependencies**: 1.4.1

**Description**: Handle structs containing other structs.

**Acceptance Criteria**:
- [ ] Detect when member type is another struct
- [ ] Recursively resolve inner struct size
- [ ] Avoid infinite recursion (cycle detection)

---

### Epic 1.5: Layout Analysis

#### Task 1.5.1: Build StructInfo Model
**Size**: S | **Priority**: P0 | **Dependencies**: 1.4.1

**Description**: Define data structures for analysis results.

**Acceptance Criteria**:
- [ ] `StructInfo` with name, size, alignment, members
- [ ] `MemberInfo` with name, offset, size, type
- [ ] `PaddingHole` with offset, size, kind
- [ ] Derive `Serialize`, `Debug`, `Clone`

```rust
#[derive(Debug, Clone, Serialize)]
pub struct StructInfo {
    pub name: String,
    pub size: usize,
    pub alignment: usize,
    pub members: Vec<MemberInfo>,
    pub source_location: Option<SourceLocation>,
}

#[derive(Debug, Clone, Serialize)]
pub struct MemberInfo {
    pub name: String,
    pub offset: usize,
    pub size: usize,
    pub type_name: String,
    pub alignment: usize,
}
```

---

#### Task 1.5.2: Implement Padding Detection
**Size**: M | **Priority**: P0 | **Dependencies**: 1.5.1

**Description**: Identify padding holes in struct layouts.

**Acceptance Criteria**:
- [ ] Sort members by offset
- [ ] Detect leading padding (before first member)
- [ ] Detect internal padding (between members)
- [ ] Detect tail padding (after last member)
- [ ] Calculate total padding bytes

---

#### Task 1.5.3: Implement Density Calculation
**Size**: XS | **Priority**: P0 | **Dependencies**: 1.5.2

**Description**: Calculate packing density metric.

**Acceptance Criteria**:
- [ ] `density = (size - padding) / size`
- [ ] Handle edge case of zero-size structs
- [ ] Store as float in StructInfo

---

### Epic 1.6: Output Formatting

#### Task 1.6.1: Implement Text Table Output
**Size**: M | **Priority**: P0 | **Dependencies**: 1.5.2

**Description**: Format struct layout as terminal table.

**Acceptance Criteria**:
- [ ] Use `comfy-table` for formatting
- [ ] Columns: Offset, Field, Size, Type
- [ ] Highlight padding rows (different color)
- [ ] Show summary line (total size, padding, density)
- [ ] Support `--no-color` flag

---

#### Task 1.6.2: Implement JSON Output
**Size**: S | **Priority**: P0 | **Dependencies**: 1.5.2

**Description**: Serialize analysis results to JSON.

**Acceptance Criteria**:
- [ ] Implement full JSON schema from spec
- [ ] Include metadata (binary hash, timestamp)
- [ ] Pretty-print with `--pretty` flag
- [ ] Validate against schema

---

#### Task 1.6.3: Implement Markdown Output
**Size**: S | **Priority**: P2 | **Dependencies**: 1.5.2

**Description**: Format output as Markdown table.

**Acceptance Criteria**:
- [ ] Standard Markdown table syntax
- [ ] Suitable for GitHub PR comments
- [ ] Include summary section

---

### Epic 1.7: CLI Interface

#### Task 1.7.1: Implement `inspect` Command
**Size**: M | **Priority**: P0 | **Dependencies**: 1.6.1, 1.6.2

**Description**: Main command for analyzing a binary.

**Acceptance Criteria**:
- [ ] Accept binary path as positional argument
- [ ] `--format` flag for output selection
- [ ] `--output` flag for file output
- [ ] `--include` and `--exclude` filters
- [ ] `--verbose` for detailed output
- [ ] `--sort` and `--limit` options

---

#### Task 1.7.2: Implement Filtering Logic
**Size**: S | **Priority**: P0 | **Dependencies**: 1.7.1

**Description**: Filter structs by regex patterns.

**Acceptance Criteria**:
- [ ] Parse regex from `--include` / `--exclude`
- [ ] Apply filters to struct names
- [ ] Support multiple patterns (OR logic for include, AND for exclude)

---

---

## Phase 2: Advanced Analysis

### Epic 2.1: Complex Location Handling

#### Task 2.1.1: Implement DWARF Expression Evaluator
**Size**: L | **Priority**: P0 | **Dependencies**: 1.3.5

**Description**: Evaluate DWARF location expressions.

**Acceptance Criteria**:
- [ ] Use `gimli::Evaluation`
- [ ] Handle `DW_OP_plus_uconst`
- [ ] Handle `DW_OP_constu`, `DW_OP_consts`
- [ ] Handle basic stack operations
- [ ] Return computed offset

---

### Epic 2.2: Bitfield Support

#### Task 2.2.1: Detect Bitfield Members
**Size**: S | **Priority**: P0 | **Dependencies**: 1.3.4

**Description**: Identify members that are bitfields.

**Acceptance Criteria**:
- [ ] Check for `DW_AT_bit_size` attribute
- [ ] Mark member as `is_bitfield = true`

---

#### Task 2.2.2: Implement DWARF 4 Bitfield Calculation
**Size**: M | **Priority**: P0 | **Dependencies**: 2.2.1

**Description**: Calculate bitfield offset for DWARF 4.

**Acceptance Criteria**:
- [ ] Extract `DW_AT_bit_offset` and `DW_AT_bit_size`
- [ ] Handle Big-Endian vs Little-Endian conversion
- [ ] Store absolute bit offset

---

#### Task 2.2.3: Implement DWARF 5 Bitfield Calculation
**Size**: S | **Priority**: P0 | **Dependencies**: 2.2.1

**Description**: Calculate bitfield offset for DWARF 5.

**Acceptance Criteria**:
- [ ] Extract `DW_AT_data_bit_offset`
- [ ] Directly use as absolute offset

---

#### Task 2.2.4: Unify Bitfield Handling
**Size**: S | **Priority**: P0 | **Dependencies**: 2.2.2, 2.2.3

**Description**: Version-aware bitfield resolution.

**Acceptance Criteria**:
- [ ] Check CU DWARF version
- [ ] Branch to appropriate calculation
- [ ] Unit tests for both versions

---

### Epic 2.3: Differential Analysis

#### Task 2.3.1: Implement Struct Matching
**Size**: M | **Priority**: P0 | **Dependencies**: 1.5.1

**Description**: Match structs between two reports by name.

**Acceptance Criteria**:
- [ ] Build hashmap of structs by fully qualified name
- [ ] Identify added, removed, changed, unchanged
- [ ] Handle namespaced names correctly

---

#### Task 2.3.2: Implement Layout Comparison
**Size**: M | **Priority**: P0 | **Dependencies**: 2.3.1

**Description**: Compare layouts of matched structs.

**Acceptance Criteria**:
- [ ] Compare size, padding, member count
- [ ] Identify specific changes (added/removed members)
- [ ] Calculate deltas

---

#### Task 2.3.3: Implement `diff` Command
**Size**: M | **Priority**: P0 | **Dependencies**: 2.3.2

**Description**: CLI command for comparing binaries.

**Acceptance Criteria**:
- [ ] Accept two binary paths
- [ ] Display diff summary
- [ ] Highlight regressions vs improvements
- [ ] Support all output formats

---

### Epic 2.4: CI Mode

#### Task 2.4.1: Implement Config File Parser
**Size**: M | **Priority**: P0 | **Dependencies**: 1.7.1

**Description**: Parse `.struct-audit.yaml`.

**Acceptance Criteria**:
- [ ] Use `serde_yaml` for parsing
- [ ] Validate schema
- [ ] Provide helpful error messages
- [ ] Support all config options from spec

---

#### Task 2.4.2: Implement Budget Validation
**Size**: M | **Priority**: P0 | **Dependencies**: 2.4.1

**Description**: Check structs against defined budgets.

**Acceptance Criteria**:
- [ ] Match structs to budget patterns
- [ ] Check max_size, max_padding constraints
- [ ] Collect violations

---

#### Task 2.4.3: Implement `check` Command
**Size**: M | **Priority**: P0 | **Dependencies**: 2.4.2

**Description**: CI command with exit codes.

**Acceptance Criteria**:
- [ ] Exit 0 on success
- [ ] Exit 1 on budget violation
- [ ] Clear error messages
- [ ] Optional baseline comparison

---

### Epic 2.5: Cache Line Analysis

#### Task 2.5.1: Implement Cache Line Metrics
**Size**: M | **Priority**: P1 | **Dependencies**: 1.5.2

**Description**: Analyze cache line efficiency.

**Acceptance Criteria**:
- [ ] Calculate cache lines spanned
- [ ] Detect straddling members
- [ ] Calculate cache utilization
- [ ] Configurable cache line size

---

---

## Phase 3: SaaS Platform

### Epic 3.1: API Backend

#### Task 3.1.1: Initialize Backend Project
**Size**: S | **Priority**: P0 | **Dependencies**: None

**Description**: Set up API server project.

**Acceptance Criteria**:
- [ ] Rust (Axum) project structure
- [ ] Health check endpoint
- [ ] Structured logging
- [ ] Docker build configuration

---

#### Task 3.1.2: Implement Database Schema
**Size**: M | **Priority**: P0 | **Dependencies**: 3.1.1

**Description**: Create PostgreSQL schema.

**Acceptance Criteria**:
- [ ] Tables as specified in architecture doc
- [ ] Migration system (sqlx/diesel migrations)
- [ ] Indexes for common queries

---

#### Task 3.1.3: Implement Report Upload Endpoint
**Size**: M | **Priority**: P0 | **Dependencies**: 3.1.2

**Description**: `POST /api/v1/reports`.

**Acceptance Criteria**:
- [ ] Validate JSON schema
- [ ] Authenticate via API token
- [ ] Store report data
- [ ] Return report ID and summary

---

#### Task 3.1.4: Implement CLI `upload` Command
**Size**: S | **Priority**: P0 | **Dependencies**: 3.1.3

**Description**: CLI command to upload reports.

**Acceptance Criteria**:
- [ ] Send report to API
- [ ] Handle auth token
- [ ] Display upload result

---

### Epic 3.2: GitHub Integration

#### Task 3.2.1: Register GitHub App
**Size**: M | **Priority**: P0 | **Dependencies**: 3.1.1

**Description**: Create and configure GitHub App.

**Acceptance Criteria**:
- [ ] App registered with required permissions
- [ ] Webhook URL configured
- [ ] OAuth flow working

---

#### Task 3.2.2: Implement Webhook Handler
**Size**: L | **Priority**: P0 | **Dependencies**: 3.2.1

**Description**: Process GitHub webhook events.

**Acceptance Criteria**:
- [ ] Verify webhook signatures
- [ ] Handle `pull_request` events
- [ ] Handle `check_run` events
- [ ] Queue processing jobs

---

#### Task 3.2.3: Implement PR Comment Posting
**Size**: M | **Priority**: P0 | **Dependencies**: 3.2.2

**Description**: Post analysis results to PRs.

**Acceptance Criteria**:
- [ ] Generate Markdown comment
- [ ] Post via GitHub API
- [ ] Update existing comments (don't spam)

---

#### Task 3.2.4: Implement Status Check Posting
**Size**: M | **Priority**: P0 | **Dependencies**: 3.2.2

**Description**: Update PR status checks.

**Acceptance Criteria**:
- [ ] Create check run on PR open
- [ ] Update status on analysis complete
- [ ] Set pass/fail based on budgets

---

### Epic 3.3: Dashboard Frontend

#### Task 3.3.1: Initialize Frontend Project
**Size**: S | **Priority**: P0 | **Dependencies**: None

**Description**: Set up Next.js project.

**Acceptance Criteria**:
- [ ] Next.js 14 with App Router
- [ ] Tailwind CSS configured
- [ ] TypeScript strict mode

---

#### Task 3.3.2: Implement Auth Flow
**Size**: M | **Priority**: P0 | **Dependencies**: 3.3.1, 3.2.1

**Description**: GitHub OAuth login.

**Acceptance Criteria**:
- [ ] Login page
- [ ] OAuth redirect handling
- [ ] Session management
- [ ] Protected routes

---

#### Task 3.3.3: Implement Repository List
**Size**: M | **Priority**: P0 | **Dependencies**: 3.3.2

**Description**: Show user's connected repositories.

**Acceptance Criteria**:
- [ ] Fetch repos from API
- [ ] Display list with basic stats
- [ ] Link to repo detail page

---

#### Task 3.3.4: Implement Struct History Chart
**Size**: L | **Priority**: P0 | **Dependencies**: 3.3.3

**Description**: Time-series chart for struct metrics.

**Acceptance Criteria**:
- [ ] Chart library integration (Chart.js/Recharts)
- [ ] Fetch history data from API
- [ ] Interactive zoom/pan
- [ ] Hover tooltips with commit info

---

#### Task 3.3.5: Implement Budget Configuration UI
**Size**: M | **Priority**: P1 | **Dependencies**: 3.3.3

**Description**: UI for managing struct budgets.

**Acceptance Criteria**:
- [ ] Form for adding/editing budgets
- [ ] Pattern validation
- [ ] Save to database

---

---

## Task Summary

| Phase | Epic Count | Task Count | Est. Days |
|-------|------------|------------|-----------|
| Phase 1 | 7 | 20 | 25-30 |
| Phase 2 | 5 | 13 | 15-20 |
| Phase 3 | 3 | 14 | 25-30 |
| **Total** | **15** | **47** | **65-80** |

---

*Previous: [Specifications](./08-spec.md) | Next: [README](./README.md)*
