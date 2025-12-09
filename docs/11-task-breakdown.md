# Task Breakdown

## Granular Implementation Tasks for struct-audit

**Context**: Solo side project - No team collaboration, no strict timeline. Work at your own pace.

---

## Priority Guide

- **P0**: MVP must-haves (build first)
- **P1**: Nice to have (after MVP)
- **P2**: Future/optional (can skip)

---

## Overview

This document breaks down the implementation roadmap into specific, actionable tasks. Each task is sized for approximately 1-4 hours of focused work. Tasks are organized by priority rather than strict timeline.

---

## Phase 1: Core CLI Development (P0 - MVP Must-Haves)

### Week 1: Project Setup

#### 1.1 Initialize Rust Workspace
- [ ] Create new Rust project: `cargo new struct-audit`
- [ ] Configure `Cargo.toml` with workspace settings
- [ ] Add initial dependencies (gimli, object, clap, serde)
- [ ] Set up rustfmt and clippy configuration
- [ ] Create `.gitignore` for Rust projects

#### 1.2 Set Up CI Pipeline
- [ ] Create `.github/workflows/ci.yml`
- [ ] Configure build matrix (Linux, macOS, Windows)
- [ ] Add cargo test step
- [ ] Add cargo clippy step
- [ ] Add cargo fmt check step
- [ ] Configure release build workflow

#### 1.3 Create Project Structure
```
src/
├── main.rs           # CLI entry point
├── lib.rs            # Library root
├── cli/
│   ├── mod.rs
│   └── commands/     # Command implementations
├── loader/
│   ├── mod.rs        # Binary loading
│   ├── elf.rs
│   ├── macho.rs
│   └── pe.rs
├── dwarf/
│   ├── mod.rs        # DWARF parsing
│   ├── context.rs
│   ├── types.rs
│   └── expression.rs
├── analysis/
│   ├── mod.rs        # Analysis engine
│   ├── padding.rs
│   ├── cache.rs
│   └── metrics.rs
├── output/
│   ├── mod.rs        # Output formatters
│   ├── table.rs
│   ├── json.rs
│   └── markdown.rs
└── error.rs          # Error types
```

#### 1.4 Define Core Data Structures
- [ ] Create `StructLayout` struct
- [ ] Create `MemberLayout` struct
- [ ] Create `LayoutMetrics` struct
- [ ] Create `PaddingHole` struct
- [ ] Implement `Display` traits for output
- [ ] Add serde derives for JSON serialization

---

### Week 2: Binary Loading

#### 2.1 Implement Binary Loader Trait
```rust
pub trait BinaryLoader {
    fn load(path: &Path) -> Result<LoadedBinary>;
    fn get_debug_sections(&self) -> DebugSections;
}
```
- [ ] Define `BinaryLoader` trait
- [ ] Define `LoadedBinary` struct
- [ ] Define `DebugSections` struct
- [ ] Add memory mapping support with `memmap2`

#### 2.2 Implement ELF Loader
- [ ] Parse ELF header using `object` crate
- [ ] Extract `.debug_info` section
- [ ] Extract `.debug_abbrev` section
- [ ] Extract `.debug_str` section
- [ ] Extract `.debug_line` section (optional)
- [ ] Handle compressed debug sections (`.zdebug_*`)
- [ ] Add tests with sample ELF binaries

#### 2.3 Implement Mach-O Loader
- [ ] Parse Mach-O header using `object` crate
- [ ] Handle universal (fat) binaries
- [ ] Extract `__debug_info` section
- [ ] Extract other debug sections
- [ ] Add tests with sample Mach-O binaries

#### 2.4 Implement PE Loader
- [ ] Parse PE header using `object` crate
- [ ] Locate embedded PDB or DWARF sections
- [ ] Handle separate `.pdb` files (future)
- [ ] Add tests with sample PE binaries

#### 2.5 Create Unified Loader
- [ ] Auto-detect binary format from magic bytes
- [ ] Create factory function `load_binary(path)`
- [ ] Add comprehensive error messages
- [ ] Test cross-platform loading

---

### Week 3: DWARF Parsing Foundation

#### 3.1 Initialize gimli Context
- [ ] Create `DwarfContext` wrapper struct
- [ ] Implement `from_sections()` constructor
- [ ] Handle endianness detection
- [ ] Set up abbreviation table loading
- [ ] Add error handling for malformed DWARF

#### 3.2 Implement Compilation Unit Iterator
```rust
pub fn iter_units(&self) -> impl Iterator<Item = CompilationUnit>
```
- [ ] Iterate over `.debug_info` headers
- [ ] Parse CU header (version, address size)
- [ ] Load abbreviation table for each CU
- [ ] Handle DWARF 4 vs DWARF 5 differences
- [ ] Add unit tests

#### 3.3 Implement DIE Traversal
- [ ] Create `find_struct_types()` function
- [ ] Filter for `DW_TAG_structure_type`
- [ ] Filter for `DW_TAG_class_type` (C++)
- [ ] Skip forward declarations (no `DW_AT_byte_size`)
- [ ] Handle nested types

#### 3.4 Extract Basic Attributes
- [ ] Implement `get_name()` - resolve `DW_AT_name`
- [ ] Implement `get_size()` - resolve `DW_AT_byte_size`
- [ ] Implement `get_alignment()` - resolve `DW_AT_alignment`
- [ ] Handle string table lookups
- [ ] Add source location extraction (file, line)

---

### Week 4: Member Extraction

#### 4.1 Parse DW_TAG_member
- [ ] Iterate children of struct DIE
- [ ] Filter for `DW_TAG_member` tags
- [ ] Extract member name
- [ ] Extract member type reference
- [ ] Handle anonymous members

#### 4.2 Resolve Constant Offsets
- [ ] Parse `DW_AT_data_member_location`
- [ ] Handle constant integer values
- [ ] Handle `DW_FORM_data*` forms
- [ ] Add tests with simple structs

#### 4.3 Implement Type Chain Resolution
```rust
pub fn resolve_type_size(type_ref: TypeOffset) -> Result<u64>
```
- [ ] Follow `DW_AT_type` references
- [ ] Handle `DW_TAG_typedef`
- [ ] Handle `DW_TAG_const_type`
- [ ] Handle `DW_TAG_pointer_type`
- [ ] Handle `DW_TAG_array_type`
- [ ] Handle `DW_TAG_base_type`
- [ ] Cache resolved types for performance

#### 4.4 Handle Nested Structs
- [ ] Detect nested struct definitions
- [ ] Generate qualified names (`Outer::Inner`)
- [ ] Handle anonymous nested structs
- [ ] Add tests with complex nesting

#### 4.5 Handle Inheritance (C++)
- [ ] Parse `DW_TAG_inheritance` DIEs
- [ ] Extract base class offset
- [ ] Handle virtual inheritance markers
- [ ] Add tests with C++ class hierarchies

---

### Week 5: Expression Evaluator

#### 5.1 Implement Location Expression Parser
- [ ] Create `ExpressionEvaluator` struct
- [ ] Parse `DW_FORM_exprloc` data
- [ ] Set up evaluation stack

#### 5.2 Implement Common Operations
- [ ] `DW_OP_constu` - push unsigned constant
- [ ] `DW_OP_consts` - push signed constant
- [ ] `DW_OP_plus_uconst` - add constant
- [ ] `DW_OP_plus` - add top two stack values
- [ ] `DW_OP_minus` - subtract
- [ ] `DW_OP_addr` - push address

#### 5.3 Handle Complex Cases
- [ ] Virtual inheritance offsets
- [ ] Multiple inheritance scenarios
- [ ] Return error for runtime-only operations
- [ ] Add comprehensive tests

#### 5.4 Integrate with Member Parsing
- [ ] Update `get_member_offset()` to use evaluator
- [ ] Fall back gracefully on unsupported expressions
- [ ] Log warnings for skipped members

---

### Week 6: Output Formatting

#### 6.1 Implement Table Formatter
- [ ] Use `comfy-table` crate
- [ ] Create layout table with columns: Offset, Size, Type, Field
- [ ] Add padding hole rows with visual indicator
- [ ] Add cache line boundary markers
- [ ] Colorize output with `colored` crate
- [ ] Show struct summary (size, padding %, cache lines)

#### 6.2 Implement JSON Formatter
- [ ] Define JSON schema (see API spec)
- [ ] Implement `Serialize` for all types
- [ ] Add `--output json` flag
- [ ] Pretty-print option
- [ ] Add tests for JSON validity

#### 6.3 Implement Markdown Formatter
- [ ] Create markdown table output
- [ ] Add summary section
- [ ] Suitable for PR comments
- [ ] Add `--output markdown` flag

#### 6.4 Create CLI Interface
```bash
struct-audit inspect <binary> [OPTIONS]
  --filter <pattern>    Filter structs by name
  --output <format>     Output format (table, json, markdown)
  --no-color            Disable colored output
  --cache-line <size>   Cache line size (default: 64)
```
- [ ] Implement `inspect` command with clap
- [ ] Add filtering by struct name (glob/regex)
- [ ] Add output format selection
- [ ] Add help text and examples

#### 6.5 Testing and Polish
- [ ] Create test binaries in multiple languages
- [ ] Compare output with pahole
- [ ] Performance benchmarks
- [ ] Fix edge cases
- [ ] Write user documentation

---

## Phase 2: Advanced Analysis & Diffing (P1 - Nice to Have)

### Week 7: DWARF 5 Bitfields

#### 7.1 Detect DWARF Version
- [ ] Read version from CU header
- [ ] Create version-aware parsing context
- [ ] Add version to output metadata

#### 7.2 Implement DWARF 4 Bitfield Handling
- [ ] Parse `DW_AT_bit_offset`
- [ ] Parse `DW_AT_bit_size`
- [ ] Handle endianness conversion
- [ ] Calculate absolute bit position

#### 7.3 Implement DWARF 5 Bitfield Handling
- [ ] Parse `DW_AT_data_bit_offset`
- [ ] Simpler calculation (endianness-independent)
- [ ] Fall back to DWARF 4 style if needed

#### 7.4 Add Bitfield Tests
- [ ] Create test structs with bitfields
- [ ] Test on GCC, Clang, MSVC output
- [ ] Test mixed bitfield/regular fields
- [ ] Verify against manual calculation

---

### Week 8: Diff Algorithm

#### 8.1 Implement Struct Matching
```rust
pub fn match_structs(old: &[StructLayout], new: &[StructLayout]) -> StructDiff
```
- [ ] Match by fully-qualified name
- [ ] Handle renamed structs (heuristic)
- [ ] Identify added structs
- [ ] Identify removed structs

#### 8.2 Implement Member Diffing
- [ ] Compare member lists
- [ ] Detect added members
- [ ] Detect removed members
- [ ] Detect reordered members
- [ ] Detect type changes

#### 8.3 Calculate Deltas
- [ ] Size delta (bytes)
- [ ] Padding delta (bytes and %)
- [ ] Cache line delta
- [ ] Density delta

#### 8.4 Create Diff Output
- [ ] Table format showing changes
- [ ] JSON format for CI parsing
- [ ] Summary statistics
- [ ] Exit code based on regression

---

### Week 9: CI Mode

#### 9.1 Implement Budget Configuration
```yaml
budgets:
  - name: "my_app::Order"
    max_size: 64
```
- [ ] Define YAML schema
- [ ] Parse with `serde_yaml`
- [ ] Support exact names and glob patterns
- [ ] Validate configuration

#### 9.2 Implement Budget Evaluation
- [ ] Match structs to budgets
- [ ] Check size constraints
- [ ] Check padding constraints
- [ ] Check cache line constraints
- [ ] Generate violation report

#### 9.3 Implement `check` Command
```bash
struct-audit check <binary> --config .struct-audit.yaml
```
- [ ] Load configuration file
- [ ] Run analysis
- [ ] Evaluate budgets
- [ ] Return exit code 0 (pass) or 1 (fail)
- [ ] Machine-readable output option

#### 9.4 Create GitHub Action
```yaml
- uses: struct-audit/action@v1
  with:
    binary: ./target/release/my_app
    config: .struct-audit.yaml
```
- [ ] Create action.yml
- [ ] Docker-based action
- [ ] Input parameters
- [ ] Output annotations
- [ ] PR comment integration

---

### Week 10: Polish & Documentation

#### 10.1 Performance Optimization
- [ ] Profile with large binaries
- [ ] Parallelize CU processing
- [ ] Optimize type resolution caching
- [ ] Reduce memory allocations

#### 10.2 Error Handling
- [ ] Graceful handling of malformed DWARF
- [ ] Clear error messages
- [ ] Suggestions for common issues
- [ ] Debug logging option

#### 10.3 Documentation
- [ ] README with quick start
- [ ] CLI help text
- [ ] Configuration reference
- [ ] Troubleshooting guide
- [ ] Example configurations

#### 10.4 Release v0.2.0
- [ ] Update CHANGELOG
- [ ] Tag release
- [ ] Publish to crates.io
- [ ] Announce on social media

---

## Phase 3: SaaS Platform MVP (P1 - Optional, Only if Monetizing)

### Weeks 11-12: API Backend

#### 11.1 Backend Setup
- [ ] Initialize Axum project
- [ ] Set up project structure
- [ ] Configure environment variables
- [ ] Set up logging (tracing)
- [ ] Create health check endpoint

#### 11.2 Database Setup
- [ ] Design PostgreSQL schema
- [ ] Create migrations (sqlx)
- [ ] Implement connection pooling
- [ ] Create repository layer

#### 11.3 Authentication
- [ ] GitHub OAuth flow
- [ ] JWT token generation
- [ ] Session management
- [ ] API key generation

#### 11.4 Report Upload API
- [ ] `POST /api/v1/reports` endpoint
- [ ] Request validation
- [ ] Struct deduplication
- [ ] Store in database
- [ ] Return report ID

#### 11.5 Query APIs
- [ ] `GET /api/v1/repos` - list repositories
- [ ] `GET /api/v1/repos/:id/structs` - list structs
- [ ] `GET /api/v1/repos/:id/structs/:name/history` - struct history
- [ ] Pagination support

---

### Weeks 13-14: GitHub Integration

#### 13.1 GitHub App Setup
- [ ] Register GitHub App
- [ ] Configure permissions
- [ ] Set up webhook URL
- [ ] Handle installation events

#### 13.2 Webhook Handler
- [ ] Receive PR events
- [ ] Verify webhook signature
- [ ] Parse event payload
- [ ] Trigger analysis

#### 13.3 PR Comments
- [ ] Generate markdown report
- [ ] Post comment via GitHub API
- [ ] Update existing comments
- [ ] Handle rate limits

#### 13.4 Check Status
- [ ] Create check run
- [ ] Update check status
- [ ] Add annotations
- [ ] Link to dashboard

---

### Weeks 15-16: Frontend Dashboard

#### 15.1 Frontend Setup
- [ ] Initialize Next.js project
- [ ] Configure Tailwind CSS
- [ ] Set up authentication (NextAuth)
- [ ] Create layout components

#### 15.2 Dashboard Pages
- [ ] Home/overview page
- [ ] Repository list
- [ ] Struct list view
- [ ] Struct detail/history view
- [ ] Settings page

#### 15.3 Visualizations
- [ ] Trend charts (Recharts)
- [ ] Struct layout diagram
- [ ] Health score indicator
- [ ] Sparklines for history

#### 15.4 Deployment
- [ ] Deploy backend to Render
- [ ] Deploy frontend to Vercel/Render
- [ ] Configure domain
- [ ] Set up SSL
- [ ] Configure monitoring

#### 15.5 Launch
- [ ] Final testing
- [ ] Write launch blog post
- [ ] Announce on Hacker News
- [ ] Monitor for issues

---

## Phase 4: Advanced Capabilities (P2 - Optional)

### False Sharing Detection

#### 16.1 Implement Atomic Type Detection
- [ ] Detect `std::atomic` types in member types
- [ ] Detect Rust `AtomicU64`, `AtomicBool`, etc.
- [ ] Mark members as atomic in `MemberInfo`

#### 16.2 Implement Cache Line Collision Detection
- [ ] Calculate cache line boundaries for atomic members
- [ ] Detect when multiple atomics share same cache line
- [ ] Generate false sharing warnings

#### 16.3 Add False Sharing Output
- [ ] Add `--detect-false-sharing` flag
- [ ] Create warning output format
- [ ] Include suggestions for padding/alignment

### Optimization Suggestions

#### 17.1 Implement Bin-Packing Heuristic
- [ ] Collect member sizes and alignments
- [ ] Implement greedy bin-packing algorithm
- [ ] Calculate optimal field order

#### 17.2 Create `suggest` Command
- [ ] Implement `struct-audit suggest <binary> --struct <name>`
- [ ] Generate optimization recommendations
- [ ] Calculate potential savings
- [ ] Handle FFI/serialization constraints
- [ ] Add warnings for breaking changes

### Go Language Support

#### 18.1 Research Go DWARF Format
- [ ] Study Go DWARF output differences
- [ ] Document Go-specific type representations
- [ ] Test with Go binaries (`-gcflags=-dwarf`)

#### 18.2 Implement Go Type Resolution
- [ ] Handle Go slice types
- [ ] Handle Go map types
- [ ] Support Go interface types
- [ ] Handle Go naming conventions

#### 18.3 Add Go-Specific Tests
- [ ] Create test Go binaries
- [ ] Verify layout accuracy
- [ ] Test edge cases

### LTO Insights

#### 19.1 Detect LTO Builds
- [ ] Detect LTO markers in binaries
- [ ] Tag reports with LTO status

#### 19.2 Compare Pre/Post LTO Layouts
- [ ] Support comparing LTO vs non-LTO builds
- [ ] Calculate LTO savings
- [ ] Identify inlined structs

#### 19.3 Add LTO Analysis Output
- [ ] Create LTO comparison format
- [ ] Show optimization effects
- [ ] Highlight structs optimized by LTO

### GitLab Integration

#### 20.1 Register GitLab App
- [ ] Create GitLab App
- [ ] Configure permissions
- [ ] Set up webhook URL

#### 20.2 Implement GitLab Webhook Handler
- [ ] Handle Merge Request events
- [ ] Verify webhook signatures
- [ ] Parse event payload

#### 20.3 Post MR Comments
- [ ] Generate markdown report
- [ ] Post comment via GitLab API
- [ ] Update existing comments

#### 20.4 Update Pipeline Status
- [ ] Create pipeline status
- [ ] Update status on analysis complete
- [ ] Handle GitLab API differences

---

## Task Tracking

Use this checklist format to track progress:

```
- [ ] Not started
- [~] In progress
- [x] Complete
- [-] Blocked/Cancelled
```

---

## Definition of Done

Each task is complete when:

1. ✅ Code is written and compiles
2. ✅ Unit tests pass
3. ✅ Code is reviewed (self or peer)
4. ✅ Documentation updated if needed
5. ✅ No new clippy warnings
6. ✅ Committed to feature branch


