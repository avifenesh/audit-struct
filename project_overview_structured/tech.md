# Technical Design

## Foundations: DWARF and Object Formats

- Use the Rust `object` crate to abstract over:
  - ELF (Linux), Mach-O (macOS), and PE (Windows).
  - Access `.debug_*` sections as byte slices independent of platform.
- Use the Rust `gimli` crate to parse DWARF debug information:
  - Zero-copy parsing to avoid excessive allocations.
  - Lazy iteration over Compilation Units (CUs) and Debugging Information Entries (DIEs).
  - Built-in support for DWARF 4 and 5.

Key DWARF concepts leveraged:

- `DW_TAG_structure_type`, `DW_TAG_member`, `DW_TAG_union_type`, `DW_TAG_inheritance`.
- Attributes:
  - `DW_AT_name`, `DW_AT_byte_size`, `DW_AT_type`.
  - `DW_AT_data_member_location`, `DW_AT_bit_size`, `DW_AT_data_bit_offset` / `DW_AT_bit_offset`.

## CLI Architecture

High-level pipeline:

1. Load:
   - Memory-map the binary if possible.
   - Extract DWARF-relevant sections into a `gimli::Dwarf` context.
2. Discover:
   - Iterate CUs.
   - Traverse DIE trees looking for struct-like tags.
3. Resolve:
   - For each struct:
     - Resolve name, total byte size, and children members.
     - For each member, resolve type, size, and location.
4. Calculate:
   - Compute:
     - Member offsets.
     - Padding gaps between members.
     - Tail padding at the end of the struct.
     - Cache-line crossings and density metrics.
5. Filter:
   - Apply user filters (e.g., regex on struct names, namespace prefixes, size thresholds).
6. Render:
   - Render either:
     - Colorized table for humans.
     - JSON structure for machines.

## Member Location Resolution

- Simple case:
  - `DW_AT_data_member_location` is a constant integer representing byte offset.
- Complex case:
  - `DW_AT_data_member_location` is a location expression.
  - Implement a minimal DWARF expression evaluator to support:
    - Stack-based bytecode operations (e.g., `DW_OP_plus_uconst`, `DW_OP_deref`).
    - Patterns common in C++ (e.g., virtual inheritance).

The evaluator does not aim to be a full debugger; it focuses on the subset needed for reliable member offset recovery.

## Bitfields and DWARF Version Handling

- DWARF 4 and earlier:
  - Use `DW_AT_bit_offset` and `DW_AT_bit_size`.
  - Bit offset is defined relative to the storage unit and endianness.
- DWARF 5:
  - Use `DW_AT_data_bit_offset` to express bit offsets from the beginning of the struct.

Design:

- Inspect the DWARF version from the CU header.
- Dispatch to version-specific bitfield handling logic to avoid incorrect padding calculations.
- Maintain a "bit cursor" for packing multiple bitfields into a single storage unit where appropriate.

## Padding Detection Algorithm

Given a struct:

- Collect members and sort them by byte offset.
- For each adjacent pair:
  - Compute the end of the previous member (`offset + size`).
  - Compute the gap to the next member’s offset.
  - Any positive gap is recorded as a padding region.
- After the last member:
  - Compare last end offset with struct total size.
  - Positive difference is tail padding.

The algorithm yields:

- Total padding bytes per struct.
- Locations and sizes of individual padding regions.

## Cache-Line Analysis

- Input:
  - Cache-line size (default 64 bytes; user-configurable).
- For each member:
  - Compute start and end lines:
    - `start_line = offset / line_size`
    - `end_line = (offset + size - 1) / line_size`
  - If `start_line != end_line`, mark the member as cache-line–straddling.
- Compute density:
  - Sum member sizes and divide by total cache-line span × line size.
  - Lower density implies more waste per cache-line fetch.

## Diff Algorithm

- Inputs:
  - Two sets of structs (`Vec<Struct>`), e.g., `baseline` and `candidate`.
- Identity:
  - Match structs by fully-qualified name (including namespace/module).
  - Optionally apply heuristic matching (e.g., high member overlap) to detect renames.
- For each matched pair:
  - Compute size delta.
  - Compute padding delta.
  - Generate a per-member diff (added/removed/offset-changed).
- Output:
  - Human-readable summary (e.g., "Order grew from 56 → 72 bytes, padding +8 bytes").
  - JSON diff for CI/SaaS.

## Implementation Stack

- Language: Rust.
- Core crates:
  - `gimli` for DWARF parsing.
  - `object` for multi-format binary support.
  - `clap` for CLI argument parsing.
  - `serde` for JSON serialization.
  - `memmap2` for efficient binary loading.
  - `comfy-table` (or similar) for table rendering.

