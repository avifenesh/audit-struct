# layout-audit

[![CI](https://github.com/avifenesh/layout-audit/actions/workflows/ci.yml/badge.svg)](https://github.com/avifenesh/layout-audit/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](layout-audit/LICENSE-MIT)

**Analyze binary memory layouts to detect padding inefficiencies.**

layout-audit parses DWARF debugging information to visualize the physical layout of data structures, detect padding holes, and analyze cache line efficiency. Find performance bugs before they hit production.

## Why layout-audit?

Modern CPUs are fast, but memory is slow. Poorly laid out structs waste cache lines and cause latency spikes. layout-audit makes memory layout issues visible and measurable.

```
struct InternalPadding (16 bytes, 37.5% padding, 1 cache line)

┌────────┬───────────┬──────┬───────┐
│ Offset │ Size      │ Type │ Field │
├────────┼───────────┼──────┼───────┤
│ 0      │ 1         │ char │ a     │
│ 1      │ [3 bytes] │ ---  │ PAD   │
│ 4      │ 4         │ int  │ b     │
│ 8      │ 1         │ char │ c     │
│ 9      │ [3 bytes] │ ---  │ PAD   │
│ 12     │ 4         │ int  │ d     │
└────────┴───────────┴──────┴───────┘

Summary: 10 useful bytes, 6 padding bytes (37.5%), cache density: 15.6%
```

## Installation

```bash
cargo install layout-audit
```

Or build from source:

```bash
git clone https://github.com/avifenesh/layout-audit.git
cd layout-audit/layout-audit
cargo build --release
```

## Quick Start

```bash
# Inspect all structs in a binary
layout-audit inspect ./target/debug/myapp

# Filter by struct name
layout-audit inspect ./target/debug/myapp --filter MyStruct

# JSON output for CI integration
layout-audit inspect ./target/debug/myapp --output json

# Compare layouts between two binaries
layout-audit diff ./old-binary ./new-binary

# Fail CI if regressions found (size or padding increased)
layout-audit diff ./old-binary ./new-binary --fail-on-regression

# Check against budget constraints from config file
layout-audit check ./target/debug/myapp --config .layout-audit.yaml
```

### Budget Config Example

Create `.layout-audit.yaml`:

```yaml
budgets:
  HotPathStruct:
    max_size: 64
    max_padding: 8
    max_padding_percent: 10.0
  CriticalData:
    max_size: 128
```

## Requirements

- Binary must be compiled with debug information (`-g` flag or `debug = true` in Cargo.toml)
- Supported formats: ELF (Linux), Mach-O (macOS), PE (Windows with DWARF)
- On macOS, use the dSYM bundle: `./binary.dSYM/Contents/Resources/DWARF/binary`
- On Windows, compile with MinGW/GCC or Clang to get DWARF debug info (MSVC PDB not supported)

## Documentation

See the [docs](./docs/) folder for detailed documentation:

- [Vision & Problem](./docs/01-vision-and-problem.md) - Why memory layout matters
- [Product Specification](./docs/03-product-specification.md) - Features and use cases
- [Technical Architecture](./docs/04-technical-architecture.md) - System design
- [DWARF Deep Dive](./docs/05-dwarf-technical-deep-dive.md) - How we parse debug info
- [Algorithms](./docs/06-algorithms.md) - Padding detection and analysis

## Use Cases

| Domain | Problem | How layout-audit helps |
|--------|---------|------------------------|
| **HFT/FinTech** | Latency-sensitive code | Catch padding waste that causes cache misses |
| **Embedded/IoT** | RAM constraints | Optimize struct sizes for memory-limited devices |
| **Gaming** | Frame stability | Prevent cache thrashing in hot loops |

## Contributing

Contributions are welcome! See the [implementation roadmap](./docs/08-implementation-roadmap.md) and [task breakdown](./docs/11-task-breakdown.md) for areas that need work.

```bash
# Run tests
cargo test

# Run with clippy
cargo clippy --all-targets

# Format code
cargo fmt
```

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](layout-audit/LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](layout-audit/LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
