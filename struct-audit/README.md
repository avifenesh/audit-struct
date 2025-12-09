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

## Usage

```bash
# Inspect all structs in a binary
struct-audit inspect ./target/debug/myapp

# Filter by struct name
struct-audit inspect ./target/debug/myapp --filter MyStruct

# JSON output
struct-audit inspect ./target/debug/myapp --output json --pretty

# Custom cache line size
struct-audit inspect ./target/debug/myapp --cache-line 128
```

## Example Output

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

## Requirements

- Binary must be compiled with debug information (`-g` flag)
- Supported formats: ELF (Linux), Mach-O (macOS)
- On macOS, use the dSYM bundle: `./binary.dSYM/Contents/Resources/DWARF/binary`

## License

MIT OR Apache-2.0
