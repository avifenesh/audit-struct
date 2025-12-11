# struct-audit

Analyze binary memory layouts to detect padding inefficiencies.

`struct-audit` parses DWARF debugging information to visualize the physical layout of data structures, detect padding holes, and analyze cache line efficiency.

## Installation

```bash
cargo install struct-audit
```

Or build from source:

```bash
cargo build --release
```

## Commands

### inspect - Analyze struct layouts

```bash
# Inspect all structs in a binary
struct-audit inspect ./target/debug/myapp

# Filter by struct name
struct-audit inspect ./target/debug/myapp --filter MyStruct

# Show top 10 structs with most padding
struct-audit inspect ./target/debug/myapp --sort-by padding --top 10

# Only show structs with at least 8 bytes of padding
struct-audit inspect ./target/debug/myapp --min-padding 8

# JSON output
struct-audit inspect ./target/debug/myapp -o json

# Custom cache line size (default: 64)
struct-audit inspect ./target/debug/myapp --cache-line 128
```

### diff - Compare layouts between binaries

```bash
# Compare struct layouts between two builds
struct-audit diff ./old-binary ./new-binary

# Filter to specific structs
struct-audit diff ./old-binary ./new-binary --filter Order

# Fail CI if any struct grew in size or padding
struct-audit diff ./old-binary ./new-binary --fail-on-regression

# JSON output for CI parsing
struct-audit diff ./old-binary ./new-binary -o json
```

### check - Enforce budget constraints

```bash
# Check structs against budget defined in config file
struct-audit check ./target/debug/myapp --config .struct-audit.yaml
```

Budget configuration (`.struct-audit.yaml`):

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
