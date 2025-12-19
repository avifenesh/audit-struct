# layout-audit

[![Crates.io](https://img.shields.io/crates/v/layout-audit.svg)](https://crates.io/crates/layout-audit)
[![Downloads](https://img.shields.io/crates/d/layout-audit.svg)](https://crates.io/crates/layout-audit)
[![CI](https://github.com/avifenesh/layout-audit/actions/workflows/ci.yml/badge.svg)](https://github.com/avifenesh/layout-audit/actions/workflows/ci.yml)
[![Coverage](https://img.shields.io/github/actions/workflow/status/avifenesh/layout-audit/ci.yml?label=coverage&branch=main&job=coverage)](https://github.com/avifenesh/layout-audit/actions/workflows/ci.yml)
[![License](https://img.shields.io/crates/l/layout-audit.svg)](https://github.com/avifenesh/layout-audit#license)
[![GitHub Marketplace](https://img.shields.io/badge/Marketplace-layout--audit-blue?logo=github)](https://github.com/marketplace/actions/layout-audit)

**Detect memory layout inefficiencies in C/C++/Rust/Go binaries.**

`layout-audit` reads DWARF debug info to visualize struct layouts, find padding, and flag cache-inefficient layouts. Great for catching regressions in CI.

## Install

- `cargo install layout-audit`
- Or download a prebuilt binary from GitHub Releases.

## Quick start

```bash
# Inspect all structs
layout-audit inspect ./target/debug/myapp

# JSON output
layout-audit inspect ./target/debug/myapp -o json

# SARIF output (for GitHub code scanning)
layout-audit inspect ./target/debug/myapp -o sarif > layout-audit.sarif
```

## Commands

- `inspect` — analyze struct layouts
- `diff` — compare two binaries (use `--fail-on-regression` in CI)
- `check` — enforce budgets from a config file
- `suggest` — propose field reordering (review for ABI/serialization impact)

## Budget config (`.layout-audit.yaml`)

```yaml
budgets:
  Order:
    max_size: 64
    max_padding: 8
    max_padding_percent: 15.0

  "hot_path::*":
    max_padding_percent: 5.0

  "*":
    max_size: 256
```

## GitHub Action

Basic usage:

```yaml
- uses: avifenesh/layout-audit@v0.5.0
  with:
    binary: ./target/debug/myapp
    command: inspect
```

SARIF (GitHub code scanning). The action uploads SARIF automatically when `output: sarif` is set. Your workflow must grant `security-events: write`.

```yaml
permissions:
  security-events: write

- uses: avifenesh/layout-audit@v0.5.0
  with:
    command: diff
    binary: ./target/debug/myapp
    baseline: ./target/debug/myapp-baseline
    output: sarif
```

### Action inputs

| Input | Description | Default |
|-------|-------------|---------|
| `binary` | Path to binary file (required) | - |
| `command` | `inspect`, `diff`, `check`, or `suggest` | `inspect` |
| `baseline` | Baseline binary for `diff` | - |
| `config` | Config file for `check` | `.layout-audit.yaml` |
| `filter` | Filter structs by name | - |
| `output` | Output format: `table`, `json`, or `sarif` | `table` |
| `sort-by` | Sort by: `name`, `size`, `padding`, `padding-pct` | `padding` |
| `top` | Show only top N structs | - |
| `min-padding` | Minimum padding bytes to show | - |
| `min-savings` | Minimum savings bytes to show (suggest) | - |
| `sort-by-savings` | Sort suggestions by savings (suggest) | `false` |
| `fail-on-regression` | Fail if layout regressed (diff) | `false` |
| `version` | layout-audit version to use | `latest` |

### Action outputs

| Output | Description |
|--------|-------------|
| `report` | The layout-audit output |
| `sarif-path` | Path to SARIF file (when `output: sarif`) |

## Requirements

- Rust **1.85+**
- Binaries must include DWARF debug info (`-g`)
- Formats: ELF (Linux), Mach-O (macOS), PE (Windows with MinGW)
- On macOS, pass the dSYM path: `./binary.dSYM/Contents/Resources/DWARF/binary`

## Go notes

Go is supported on Linux/macOS (Windows uses PDB). Use full debug info:

```bash
go build -gcflags=all="-N -l" -o myapp
```

Runtime types are filtered by default; use `--include-go-runtime` to show them.

## License

MIT OR Apache-2.0
