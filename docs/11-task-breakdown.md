# Task Breakdown

## Granular Implementation Tasks for struct-audit

**Context**: Solo side project. Focus on shipping a working MVP before expanding scope.

---

## Priority Guide

- **P0**: MVP must-haves (build first)
- **P1**: Post-MVP enhancements
- **P2**: Future/optional

---

## Quick Wins (Start Here)

These can be done in short sessions. Build momentum before tackling complex DWARF parsing.

- [ ] Initialize Rust workspace with `cargo new struct-audit`
- [ ] Add core dependencies (gimli, object, clap, serde)
- [ ] Set up rustfmt.toml and clippy configuration
- [ ] Create `.gitignore` for Rust projects
- [ ] Create basic `StructLayout` and `MemberLayout` structs with serde derives
- [ ] Set up basic CLI structure with clap (`inspect` subcommand skeleton)
- [ ] Create test binary corpus (simple C/Rust structs)

---

## Phase 1: Core CLI MVP (P0)

**Goal**: Ship `struct-audit inspect <binary>` for ELF binaries with constant member offsets.

**Explicit Non-Goals for MVP**:
- Mach-O/PE support (P1)
- Expression evaluator for complex offsets (P1)
- DWARF 5 bitfield handling (P1)
- Diff command (P1)
- CI check command (P1)
- SaaS anything (P2)

### 1.1 Project Setup

#### Initialize Workspace
- [ ] Create project: `cargo new struct-audit`
- [ ] Configure `Cargo.toml`:
```toml
[dependencies]
gimli = "0.28"
object = "0.32"
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
memmap2 = "0.9"
comfy-table = "7"
colored = "2"
thiserror = "1"
anyhow = "1"

[dev-dependencies]
tempfile = "3"
```
- [ ] Set up rustfmt and clippy config
- [ ] Create `.gitignore`

#### Create Project Structure
```
src/
├── main.rs           # CLI entry point
├── lib.rs            # Library root
├── cli.rs            # clap definitions
├── loader.rs         # Binary loading (ELF only for MVP)
├── dwarf/
│   ├── mod.rs        # DWARF parsing
│   ├── context.rs    # gimli wrapper
│   └── types.rs      # Type resolution
├── analysis/
│   ├── mod.rs        # Analysis engine
│   └── padding.rs    # Padding detection
├── output/
│   ├── mod.rs        # Output formatters
│   ├── table.rs      # Table output
│   └── json.rs       # JSON output
└── error.rs          # Error types

tests/
├── fixtures/         # Test binaries (C, Rust)
└── integration/      # Integration tests
```

#### Define Core Data Structures
- [ ] Create `StructLayout` struct
- [ ] Create `MemberLayout` struct
- [ ] Create `LayoutMetrics` struct
- [ ] Create `PaddingHole` struct
- [ ] Add serde derives for JSON serialization
- [ ] Implement `Display` traits

---

### 1.2 Test Infrastructure (Do Early!)

**Why early**: DWARF edge cases are discovered via testing. Can't validate without test corpus.

#### Create Test Binary Corpus
- [ ] Create `tests/fixtures/` directory
- [ ] Write simple C test structs (`test_simple.c`):
  - Struct with no padding
  - Struct with internal padding
  - Struct with tail padding
  - Nested struct
- [ ] Write Rust test structs (`test_simple.rs`):
  - Basic struct
  - `#[repr(C)]` struct
  - Struct with Option/enum
- [ ] Create Makefile/build script to compile test binaries with debug info
- [ ] Add test binaries to `.gitignore` (compile on demand)

#### Set Up Integration Tests
- [ ] Create `tests/integration/` directory
- [ ] Write test that parses simple C struct and validates layout
- [ ] Write test comparing output against known-good values
- [ ] Add CI step to run tests

---

### 1.3 Binary Loading (ELF Only)

#### Implement ELF Loader
- [ ] Create `loader.rs` with `load_binary()` function
- [ ] Use `object` crate to parse ELF header
- [ ] Memory-map file with `memmap2`
- [ ] Extract `.debug_info` section
- [ ] Extract `.debug_abbrev` section
- [ ] Extract `.debug_str` section
- [ ] Handle missing debug info gracefully (clear error message)
- [ ] Add tests with fixture binaries

---

### 1.4 DWARF Parsing Foundation

#### Initialize gimli Context
- [ ] Create `DwarfContext` wrapper struct in `dwarf/context.rs`
- [ ] Implement `from_sections()` constructor
- [ ] Handle endianness detection
- [ ] Set up abbreviation table loading
- [ ] Add error handling for malformed DWARF

#### Implement Compilation Unit Iterator
- [ ] Create `iter_units()` method
- [ ] Parse CU header (version, address size)
- [ ] Load abbreviation table for each CU
- [ ] Add unit tests

#### Implement Struct Finder
- [ ] Create `find_struct_types()` function
- [ ] Filter for `DW_TAG_structure_type`
- [ ] Filter for `DW_TAG_class_type` (C++)
- [ ] Skip forward declarations (no `DW_AT_byte_size`)
- [ ] Skip compiler-generated types (names starting with `__`)

#### Extract Basic Attributes
- [ ] Implement `get_name()` - resolve `DW_AT_name`
- [ ] Implement `get_size()` - resolve `DW_AT_byte_size`
- [ ] Implement `get_alignment()` - resolve `DW_AT_alignment` (optional attribute)
- [ ] Handle string table lookups
- [ ] Add source location extraction (file, line) - optional for MVP

---

### 1.5 Member Extraction (Constant Offsets Only)

**MVP Scope**: Only handle `DW_AT_data_member_location` as constant integer. Report "offset unavailable" for expression-based offsets.

#### Parse DW_TAG_member
- [ ] Iterate children of struct DIE
- [ ] Filter for `DW_TAG_member` tags
- [ ] Extract member name
- [ ] Extract member type reference
- [ ] Handle anonymous members (generate synthetic name)

#### Resolve Constant Offsets
- [ ] Parse `DW_AT_data_member_location`
- [ ] Handle constant integer values (`DW_FORM_data*`, `DW_FORM_udata`, `DW_FORM_sdata`)
- [ ] **For expression-based offsets**: Log warning, mark offset as `None`
- [ ] Add tests with simple structs

#### Implement Basic Type Resolution
- [ ] Follow `DW_AT_type` references
- [ ] Handle `DW_TAG_base_type` (get size directly)
- [ ] Handle `DW_TAG_pointer_type` (size = address size)
- [ ] Handle `DW_TAG_typedef` (follow chain)
- [ ] Handle `DW_TAG_const_type` (follow chain)
- [ ] Handle `DW_TAG_array_type` (element size × count)
- [ ] Cache resolved types for performance
- [ ] **Skip for MVP**: Complex template types, report size as "unknown"

---

### 1.6 Padding Detection

#### Implement Padding Algorithm
- [ ] Sort members by offset
- [ ] Detect gaps between consecutive members
- [ ] Detect tail padding (struct size - end of last member)
- [ ] Create `PaddingHole` records
- [ ] Calculate total padding bytes and percentage

#### Implement Cache Line Analysis
- [ ] Calculate cache lines spanned (size / 64, rounded up)
- [ ] Calculate density (useful bytes / total cache line bytes)
- [ ] Detect members straddling cache line boundaries
- [ ] Make cache line size configurable (default 64)

---

### 1.7 Output Formatting

#### Implement Table Formatter
- [ ] Use `comfy-table` crate
- [ ] Create layout table: Offset | Size | Type | Field
- [ ] Add padding hole rows with visual indicator (`[N bytes]  PAD`)
- [ ] Add cache line boundary markers
- [ ] Colorize with `colored` crate (padding in yellow, violations in red)
- [ ] Show struct summary header (name, size, padding %, cache lines)

#### Implement JSON Formatter
- [ ] Define output schema (see API spec doc)
- [ ] Implement `Serialize` for all types
- [ ] Add `--output json` flag
- [ ] Add `--pretty` flag for indented output

#### Create CLI Interface
```bash
struct-audit inspect <binary> [OPTIONS]
  --filter <pattern>    Filter structs by name (substring match)
  --output <format>     Output format: table (default), json
  --no-color            Disable colored output
  --cache-line <size>   Cache line size in bytes (default: 64)
```
- [ ] Implement with clap derive macros
- [ ] Add `--help` text with examples
- [ ] Add `--version` flag

---

### 1.8 MVP Polish

#### Error Handling
- [ ] Clear error messages for common issues:
  - Binary not found
  - Not an ELF file
  - No debug info (suggest compiling with `-g`)
  - Malformed DWARF
- [ ] Exit code 0 on success, 1 on error

#### Documentation
- [ ] Write README with:
  - Installation (`cargo install struct-audit`)
  - Quick start example
  - Output explanation
- [ ] Add `--help` examples

#### CI Setup
- [ ] Create `.github/workflows/ci.yml`
- [ ] Build on Linux only (MVP)
- [ ] Run `cargo test`
- [ ] Run `cargo clippy`
- [ ] Run `cargo fmt --check`

---

### 1.9 MVP Release Checklist

**v0.1.0-alpha**:
- [ ] All Phase 1 tasks complete
- [ ] Tests passing
- [ ] README written
- [ ] `cargo publish --dry-run` succeeds
- [ ] Tag release

**Success Criteria**:
- [ ] Parses test binaries without panics
- [ ] Output matches manual inspection for test structs
- [ ] Runs on real-world binary (your own Rust project)

---

## Phase 2: Enhanced CLI (P1 - After MVP)

Only start Phase 2 after MVP is shipped and used on real projects.

### 2.1 Expression Evaluator

For C++ virtual inheritance and complex layouts.

- [ ] Create `ExpressionEvaluator` struct
- [ ] Implement `DW_OP_constu`, `DW_OP_plus_uconst`
- [ ] Implement `DW_OP_plus`, `DW_OP_minus`
- [ ] Return error for runtime-only ops (`DW_OP_deref`, `DW_OP_reg*`)
- [ ] Integrate with member offset parsing
- [ ] Add tests with C++ binaries

### 2.2 DWARF 5 Bitfields

- [ ] Detect DWARF version from CU header
- [ ] Implement DWARF 4: `DW_AT_bit_offset` + `DW_AT_bit_size`
- [ ] Implement DWARF 5: `DW_AT_data_bit_offset`
- [ ] Add bitfield tests

### 2.3 Cross-Platform Support

- [ ] Implement Mach-O loader (macOS)
- [ ] Implement PE loader (Windows)
- [ ] Handle compressed debug sections (`.zdebug_*`)
- [ ] Add CI matrix for Linux/macOS/Windows

### 2.4 Diff Command

```bash
struct-audit diff <old-binary> <new-binary>
```
- [ ] Match structs by fully-qualified name
- [ ] Detect added/removed/changed structs
- [ ] Calculate size and padding deltas
- [ ] Flag cache line boundary changes
- [ ] Return exit code 1 on regression

### 2.5 CI Check Command

```bash
struct-audit check <binary> --config .struct-audit.yaml
```
- [ ] Parse YAML config with `serde_yaml`
- [ ] Support exact names and glob patterns
- [ ] Evaluate size, padding %, cache line budgets
- [ ] Return exit code 0 (pass) or 1 (fail)
- [ ] Machine-readable JSON output

### 2.6 GitHub Action

- [ ] Create `action.yml`
- [ ] Docker-based action
- [ ] Input parameters (binary path, config path)
- [ ] Output annotations

---

## Phase 3: Advanced Analysis (P1)

### 3.1 False Sharing Detection

- [ ] Detect atomic types in member type names
- [ ] Group atomics by cache line
- [ ] Flag when multiple atomics share cache line
- [ ] Add `--detect-false-sharing` flag
- [ ] Generate recommendations

### 3.2 Optimization Suggestions

```bash
struct-audit suggest <binary> --struct <name>
```
- [ ] Implement greedy bin-packing for optimal field order
- [ ] Show before/after comparison
- [ ] Calculate potential savings
- [ ] Warn about FFI/serialization implications

### 3.3 C++ Inheritance Support

- [ ] Parse `DW_TAG_inheritance` DIEs
- [ ] Extract base class offset
- [ ] Handle multiple inheritance
- [ ] Handle virtual inheritance (requires expression evaluator)

---

## Phase 4: SaaS Platform (P2 - Only If Monetizing)

**Deferred**. Focus on CLI first.

Do not start until:
1. CLI v0.2.0 is stable
2. CLI has real users (not just you)
3. You feel recurring pain from lack of historical tracking
4. You're willing to maintain hosted infrastructure

---

## Task Tracking

```
- [ ] Not started
- [~] In progress
- [x] Complete
- [-] Skipped/Cancelled
```

---

## Definition of Done

Each task is complete when:

1. Code compiles without warnings
2. Tests pass (if applicable)
3. No new clippy warnings
4. Committed to branch

Each phase is complete when:

1. All P0 tasks in phase complete
2. README updated
3. Version bumped
4. Tag created

---

## Quick Reference: What's In vs Out for MVP

| Feature | MVP (v0.1.0) | Post-MVP |
|---------|--------------|----------|
| ELF binaries | Yes | - |
| Mach-O/PE | No | v0.2.0 |
| Constant member offsets | Yes | - |
| Expression-based offsets | No (show "unknown") | v0.2.0 |
| Basic type resolution | Yes | - |
| Complex templates | No (show "unknown") | v0.2.0 |
| Padding detection | Yes | - |
| Cache line analysis | Yes | - |
| Table output | Yes | - |
| JSON output | Yes | - |
| `inspect` command | Yes | - |
| `diff` command | No | v0.2.0 |
| `check` command | No | v0.2.0 |
| Bitfields | No | v0.2.0 |
| False sharing | No | v0.3.0 |
| SaaS | No | TBD |

---

## Navigation

- [Implementation Roadmap](./08-implementation-roadmap.md) - Phase timeline
- [Technical Architecture](./04-technical-architecture.md) - System design
- [DWARF Deep Dive](./05-dwarf-technical-deep-dive.md) - Parsing details
