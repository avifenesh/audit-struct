# layout-audit

Analyze binary memory layouts to detect padding inefficiencies.

`layout-audit` parses DWARF debugging information to visualize the physical layout of data structures, detect padding holes, and analyze cache line efficiency.

## Installation

```bash
cargo install layout-audit
```

Or build from source:

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
  # Enforce constraints on specific structs
  Order:
    max_size: 64           # Maximum total size in bytes
    max_padding: 8         # Maximum padding bytes
    max_padding_percent: 15.0  # Maximum padding percentage

  CriticalPath:
    max_size: 128
    max_padding_percent: 10.0
```

Exit code 1 if any budget is exceeded (useful for CI).

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

## CI Integration

Add layout-audit to your GitHub Actions workflow:

```yaml
name: Memory Layout Check

on: [push, pull_request]

jobs:
  layout-audit:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Install layout-audit
        run: cargo install layout-audit

      - name: Build with debug info
        run: cargo build  # debug profile includes DWARF by default

      - name: Check struct budgets
        run: layout-audit check ./target/debug/myapp --config .layout-audit.yaml

      # Optional: Compare against main branch
      - name: Diff against baseline
        if: github.event_name == 'pull_request'
        run: |
          # Build baseline from main
          git fetch origin main
          git checkout origin/main -- .
          cargo build --target-dir target-baseline
          git checkout -
          # Compare
          layout-audit diff ./target-baseline/debug/myapp ./target/debug/myapp --fail-on-regression
```

## Requirements

- Binary must be compiled with debug information (`-g` flag)
- Supported formats: ELF (Linux), Mach-O (macOS), PE (Windows with MinGW)
- On macOS, use the dSYM bundle: `./binary.dSYM/Contents/Resources/DWARF/binary`

## Language Support

- C: Full support
- C++: Full support including templates
- Rust: Full support

## License

MIT OR Apache-2.0
