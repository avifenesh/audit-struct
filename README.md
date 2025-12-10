# struct-audit

[![CI](https://github.com/avifenesh/audit-struct/actions/workflows/ci.yml/badge.svg)](https://github.com/avifenesh/audit-struct/actions/workflows/ci.yml)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue.svg)](LICENSE)

**Analyze binary memory layouts to detect padding inefficiencies.**

struct-audit parses DWARF debugging information to visualize the physical layout of data structures, detect padding holes, and analyze cache line efficiency. Find performance bugs before they hit production.

## Why struct-audit?

Modern CPUs are fast, but memory is slow. Poorly laid out structs waste cache lines and cause latency spikes. struct-audit makes memory layout issues visible and measurable.

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
cargo install struct-audit
```

Or build from source:

```bash
git clone https://github.com/avifenesh/audit-struct.git
cd audit-struct/struct-audit
cargo build --release
```

## Quick Start

```bash
# Inspect all structs in a binary
struct-audit inspect ./target/debug/myapp

# Filter by struct name
struct-audit inspect ./target/debug/myapp --filter MyStruct

# JSON output for CI integration
struct-audit inspect ./target/debug/myapp --output json

# Compare layouts between two binaries
struct-audit diff ./old-binary ./new-binary

# CI mode - fail if padding exceeds budget
struct-audit check ./target/debug/myapp --max-padding 20
```

## Requirements

- Binary must be compiled with debug information (`-g` flag or `debug = true` in Cargo.toml)
- Supported formats: ELF (Linux), Mach-O (macOS)
- On macOS, use the dSYM bundle: `./binary.dSYM/Contents/Resources/DWARF/binary`

## Documentation

See the [docs](./docs/) folder for detailed documentation:

- [Vision & Problem](./docs/01-vision-and-problem.md) - Why memory layout matters
- [Product Specification](./docs/03-product-specification.md) - Features and use cases
- [Technical Architecture](./docs/04-technical-architecture.md) - System design
- [DWARF Deep Dive](./docs/05-dwarf-technical-deep-dive.md) - How we parse debug info
- [Algorithms](./docs/06-algorithms.md) - Padding detection and analysis

## Use Cases

| Domain | Problem | How struct-audit helps |
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

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
