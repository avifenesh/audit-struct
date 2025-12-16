# layout-audit

[![Crates.io](https://img.shields.io/crates/v/layout-audit.svg)](https://crates.io/crates/layout-audit)
[![Downloads](https://img.shields.io/crates/d/layout-audit.svg)](https://crates.io/crates/layout-audit)
[![CI](https://github.com/avifenesh/layout-audit/actions/workflows/ci.yml/badge.svg)](https://github.com/avifenesh/layout-audit/actions/workflows/ci.yml)
[![License](https://img.shields.io/crates/l/layout-audit.svg)](https://github.com/avifenesh/layout-audit#license)
[![GitHub Marketplace](https://img.shields.io/badge/Marketplace-layout--audit-blue?logo=github)](https://github.com/marketplace/actions/layout-audit)

**Detect memory layout inefficiencies in your C/C++/Rust/Go binaries.**

`layout-audit` parses DWARF debugging information to visualize the physical layout of data structures, detect padding holes, and analyze cache line efficiency.

## Why?

Every byte of padding costs you:
- **HFT/Trading**: Cache misses add microseconds of latency
- **Embedded/IoT**: Wasted RAM on memory-constrained devices
- **Gaming**: Poor cache utilization hurts frame times
- **Cloud**: Larger memory footprint = higher costs

Catch layout regressions in CI before they ship.

## Installation

### Pre-built binaries

Download from [GitHub Releases](https://github.com/avifenesh/layout-audit/releases/latest):

```bash
# Linux (x86_64)
curl -LO https://github.com/avifenesh/layout-audit/releases/latest/download/layout-audit-linux-x86_64
chmod +x layout-audit-linux-x86_64
sudo mv layout-audit-linux-x86_64 /usr/local/bin/layout-audit

# macOS (Apple Silicon)
curl -LO https://github.com/avifenesh/layout-audit/releases/latest/download/layout-audit-macos-aarch64
chmod +x layout-audit-macos-aarch64
sudo mv layout-audit-macos-aarch64 /usr/local/bin/layout-audit
```

### Via Cargo

```bash
cargo install layout-audit
```

### From source

```bash
cargo build --release
```

## Commands

### inspect - Analyze struct layouts

```bash
# Inspect all structs in a binary
layout-audit inspect ./target/debug/myapp

# Filter by struct name
layout-audit inspect ./target/debug/myapp --filter MyStruct

# Show top 10 structs with most padding
layout-audit inspect ./target/debug/myapp --sort-by padding --top 10

# Only show structs with at least 8 bytes of padding
layout-audit inspect ./target/debug/myapp --min-padding 8

# JSON output
layout-audit inspect ./target/debug/myapp -o json

# Custom cache line size (default: 64)
layout-audit inspect ./target/debug/myapp --cache-line 128
```

### diff - Compare layouts between binaries

```bash
# Compare struct layouts between two builds
layout-audit diff ./old-binary ./new-binary

# Filter to specific structs
layout-audit diff ./old-binary ./new-binary --filter Order

# Fail CI if any struct grew in size or padding
layout-audit diff ./old-binary ./new-binary --fail-on-regression

# JSON output for CI parsing
layout-audit diff ./old-binary ./new-binary -o json
```

### check - Enforce budget constraints

```bash
# Check structs against budget defined in config file
layout-audit check ./target/debug/myapp --config .layout-audit.yaml
```

Budget configuration (`.layout-audit.yaml`):

```yaml
budgets:
  # Exact match - highest priority
  Order:
    max_size: 64           # Maximum total size in bytes
    max_padding: 8         # Maximum padding bytes
    max_padding_percent: 15.0  # Maximum padding percentage

  # Glob patterns - matched in declaration order
  "hot_path::*":           # All structs in hot_path module
    max_size: 64
    max_padding_percent: 5.0

  "*Padding":              # Structs ending with "Padding"
    max_padding: 4

  "*":                     # Catch-all for remaining structs
    max_size: 256
```

**Pattern matching rules:**
- Exact names take priority over glob patterns
- First matching glob wins (declaration order matters)
- Supports `*` (any chars), `?` (single char), `[...]` (char class)

Exit code 1 if any budget is exceeded (useful for CI).

### suggest - Recommend optimal field ordering

```bash
# Suggest optimal field ordering for all structs
layout-audit suggest ./target/debug/myapp

# Filter to specific structs
layout-audit suggest ./target/debug/myapp --filter Order

# Only show structs with at least 8 bytes of potential savings
layout-audit suggest ./target/debug/myapp --min-savings 8

# Sort by savings (largest first)
layout-audit suggest ./target/debug/myapp --sort-by-savings

# JSON output
layout-audit suggest ./target/debug/myapp -o json
```

Example output:
```
struct InternalPadding (16 bytes -> 12 bytes, saves 4 bytes / 25.0%)

Current layout:
| Offset | Size | Align | Type | Field |
|--------|------|-------|------|-------|
| 0      | 1    | 1     | char | a     |
| 4      | 4    | 4     | int  | b     |
| 8      | 1    | 1     | char | c     |
| 12     | 4    | 4     | int  | d     |

Suggested layout:
| Offset | Size | Align | Type | Field |
|--------|------|-------|------|-------|
| 0      | 4    | 4     | int  | b     |
| 4      | 4    | 4     | int  | d     |
| 8      | 1    | 1     | char | a     |
| 9      | 1    | 1     | char | c     |

Reordering may affect serialization/FFI compatibility
```

## Example Output

```
struct InternalPadding (16 bytes, 37.5% padding, 1 cache line)

┌────────┬───────────┬──────┬───────┐
│ Offset ┆ Size      ┆ Type ┆ Field │
╞════════╪═══════════╪══════╪═══════╡
│ 0      ┆ 1         ┆ char ┆ a     │
│ 1      ┆ [3 bytes] ┆ ---  ┆ PAD   │
│ 4      ┆ 4         ┆ int  ┆ b     │
│ 8      ┆ 1         ┆ char ┆ c     │
│ 9      ┆ [3 bytes] ┆ ---  ┆ PAD   │
│ 12     ┆ 4         ┆ int  ┆ d     │
└────────┴───────────┴──────┴───────┘

Summary: 10 useful bytes, 6 padding bytes (37.5%), cache density: 15.6%
```

## GitHub Action

Use layout-audit directly in your workflows:

```yaml
- uses: avifenesh/layout-audit@v0.4.1
  with:
    binary: ./target/debug/myapp
    command: inspect
```

### Action Inputs

| Input | Description | Default |
|-------|-------------|---------|
| `binary` | Path to binary file (required) | - |
| `command` | `inspect`, `diff`, `check`, or `suggest` | `inspect` |
| `baseline` | Baseline binary for `diff` command | - |
| `config` | Config file for `check` command | `.layout-audit.yaml` |
| `filter` | Filter structs by name | - |
| `output` | Output format: `table` or `json` | `table` |
| `sort-by` | Sort by: `name`, `size`, `padding`, `padding-pct` | `padding` |
| `top` | Show only top N structs | - |
| `min-padding` | Minimum padding bytes to show | - |
| `fail-on-regression` | Fail if layout regressed (for `diff`) | `false` |
| `version` | layout-audit version to use | `latest` |

### Action Outputs

| Output | Description |
|--------|-------------|
| `report` | The layout-audit output |

### Examples

#### Inspect structs with most padding

```yaml
name: Memory Layout Check
on: [push, pull_request]

jobs:
  layout-audit:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable

      - name: Build with debug info
        run: cargo build

      - name: Analyze memory layouts
        uses: avifenesh/layout-audit@v0.4.1
        with:
          binary: ./target/debug/myapp
          command: inspect
          sort-by: padding
          top: '10'
```

#### Check budget constraints

```yaml
      - name: Check struct budgets
        uses: avifenesh/layout-audit@v0.4.1
        with:
          binary: ./target/debug/myapp
          command: check
          config: .layout-audit.yaml
```

#### Diff against baseline (fail on regression)

```yaml
      - name: Compare layouts
        uses: avifenesh/layout-audit@v0.4.1
        with:
          binary: ./target/debug/myapp
          baseline: ./target-baseline/debug/myapp
          command: diff
          fail-on-regression: 'true'
```

#### Use output in subsequent steps

```yaml
      - name: Analyze layouts
        id: layout
        uses: avifenesh/layout-audit@v0.4.1
        with:
          binary: ./target/debug/myapp
          output: json

      - name: Process results
        run: echo '${{ steps.layout.outputs.report }}' | jq '.structs | length'
```

## Requirements

- **Rust 1.85+** (MSRV)
- Binary must be compiled with debug information (`-g` flag)
- Supported formats: ELF (Linux), Mach-O (macOS), PE (Windows with MinGW)
- On macOS, use the dSYM bundle: `./binary.dSYM/Contents/Resources/DWARF/binary`

## Limitations

- Structs with identical names across compilation units are deduplicated by name in diff output

## Language Support

- C: Full support
- C++: Full support including templates
- Rust: Full support
- Go: Full support on Linux/macOS (runtime types filtered by default; Windows not supported - Go uses PDB, not DWARF)

### Go Binaries

Go binaries include DWARF debug info by default. For full debug information:

```bash
go build -gcflags=all="-N -l" -o myapp
```

Go runtime internal types (`runtime.*`, `sync.*`, etc.) are filtered out by default. Use `--include-go-runtime` to show them:

```bash
# Show only user-defined structs (default)
layout-audit inspect ./myapp --filter main.

# Include runtime types
layout-audit inspect ./myapp --include-go-runtime
```

## License

MIT OR Apache-2.0
