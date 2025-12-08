# Technical Foundations: DWARF & Binary Analysis

## Executive Summary

struct-audit's core capability depends on parsing **DWARF debugging information**—the compiler-generated metadata that describes the relationship between source code and binary layout. This document details the technical foundations required for accurate analysis.

---

## DWARF Overview

### What is DWARF?

**DWARF** (Debugging With Attributed Record Formats) is a standardized debugging data format used by compilers to embed source-level information into compiled binaries.

```
┌─────────────────────────────────────────────────────────┐
│                     ELF Binary                          │
├─────────────────────────────────────────────────────────┤
│  .text          (executable code)                       │
│  .data          (initialized data)                      │
│  .bss           (uninitialized data)                    │
├─────────────────────────────────────────────────────────┤
│  .debug_info    ◄── DWARF: Type definitions, layouts    │
│  .debug_abbrev  ◄── DWARF: Abbreviation tables          │
│  .debug_str     ◄── DWARF: String table                 │
│  .debug_line    ◄── DWARF: Line number mappings         │
│  .debug_ranges  ◄── DWARF: Address ranges               │
└─────────────────────────────────────────────────────────┘
```

### DWARF Structure

DWARF is organized hierarchically:

```
Compilation Unit (CU)
└── DIE: DW_TAG_compile_unit
    ├── DIE: DW_TAG_base_type (int, char, etc.)
    ├── DIE: DW_TAG_structure_type ◄── Our target
    │   ├── DIE: DW_TAG_member (field 1)
    │   ├── DIE: DW_TAG_member (field 2)
    │   └── DIE: DW_TAG_member (field 3)
    ├── DIE: DW_TAG_subprogram (functions)
    └── ...
```

**DIE** = Debugging Information Entry

---

## Critical DWARF Tags for struct-audit

### Structure Analysis Tags

| Tag | Purpose | Example |
|-----|---------|---------|
| `DW_TAG_structure_type` | C struct, C++ class, Rust struct | `struct Order { ... }` |
| `DW_TAG_union_type` | Union types | `union Value { ... }` |
| `DW_TAG_member` | Field within struct/union | `u64 price;` |
| `DW_TAG_inheritance` | C++ base class | `class Derived : Base` |
| `DW_TAG_class_type` | C++ class (synonym) | `class Widget { ... }` |

### Critical Attributes

| Attribute | Contains | Complexity |
|-----------|----------|------------|
| `DW_AT_name` | Type/member name | Offset into .debug_str |
| `DW_AT_byte_size` | Total size in bytes | Direct value |
| `DW_AT_data_member_location` | Field byte offset | **Can be expression** |
| `DW_AT_type` | Reference to field type | DIE offset (requires resolution) |
| `DW_AT_bit_size` | Bitfield size | Direct value |
| `DW_AT_bit_offset` | Bitfield position (DWARF 4) | **Endian-dependent** |
| `DW_AT_data_bit_offset` | Bitfield position (DWARF 5) | Direct value |

---

## Parsing Complexity: Location Expressions

### The Challenge

`DW_AT_data_member_location` is often a simple constant (e.g., "offset 8"). However, DWARF allows this to be a **Location Expression**—a stack-based bytecode program.

### When Expressions Occur

1. **Virtual Inheritance (C++)**: Base class offset depends on vtable
2. **Complex Unions**: Discriminant-based layouts
3. **Compiler Optimizations**: Unusual layouts

### Expression Evaluation

```
Example Expression: DW_OP_plus_uconst(8)

Stack Machine:
1. Push object address (implicit)
2. DW_OP_plus_uconst(8) → Add 8 to top of stack
3. Result = offset 8
```

**Implementation Requirement**: struct-audit must implement a DWARF expression evaluator. The `gimli` crate provides `gimli::Evaluation` for this purpose.

### Common Operations

| Operation | Description |
|-----------|-------------|
| `DW_OP_plus_uconst(N)` | Add constant N |
| `DW_OP_deref` | Dereference address (rare in layout) |
| `DW_OP_constu(N)` | Push constant N |
| `DW_OP_dup` | Duplicate top of stack |

---

## The Bitfield Challenge: DWARF 4 vs DWARF 5

### DWARF 4 (Legacy)

Uses `DW_AT_bit_offset` with **Big-Endian bias**:

```
For Big-Endian:    offset from START of storage unit
For Little-Endian: offset from END of storage unit (counter-intuitive!)

Storage Unit (32-bit):
┌────────────────────────────────────────┐
│ bit 31                          bit 0  │
└────────────────────────────────────────┘
         ▲
    DW_AT_bit_offset = bits from MSB
```

### DWARF 5 (Modern)

Introduces `DW_AT_data_bit_offset`:

```
Always: offset from START of containing entity

Struct:
┌────────────────────────────────────────┐
│ byte 0  │ byte 1  │ byte 2  │ byte 3   │
└────────────────────────────────────────┘
▲
DW_AT_data_bit_offset = bits from struct start
```

### Implementation Strategy

```rust
fn get_bitfield_offset(die: &DIE, cu: &CompilationUnit) -> u64 {
    let dwarf_version = cu.header.version();

    if dwarf_version >= 5 {
        // DWARF 5: Use data_bit_offset (straightforward)
        die.attr(DW_AT_data_bit_offset)
    } else {
        // DWARF 4: Convert bit_offset based on endianness
        let bit_offset = die.attr(DW_AT_bit_offset);
        let bit_size = die.attr(DW_AT_bit_size);
        let storage_size = die.attr(DW_AT_byte_size) * 8;

        if target_is_little_endian() {
            storage_size - bit_offset - bit_size
        } else {
            bit_offset
        }
    }
}
```

**Critical**: Version detection is mandatory. Incorrect bitfield handling destroys trust in the tool.

---

## Type Resolution

### The Problem

Member types are stored as **DIE references**, not inline definitions:

```
DW_TAG_structure_type "Order"
├── DW_TAG_member "id"
│   └── DW_AT_type = <0x1234>  ◄── Reference to another DIE
└── DW_TAG_member "price"
    └── DW_AT_type = <0x5678>  ◄── Reference to another DIE
```

### Resolution Chain

```
DW_AT_type → DW_TAG_typedef → DW_TAG_const_type → DW_TAG_base_type
                                                         │
                                                    DW_AT_byte_size = 8
```

**Implementation**: Must follow reference chain until reaching a type with `DW_AT_byte_size`.

### Type Categories

| Tag | Treatment |
|-----|-----------|
| `DW_TAG_base_type` | Direct size |
| `DW_TAG_pointer_type` | Architecture pointer size |
| `DW_TAG_array_type` | Element size × count |
| `DW_TAG_typedef` | Follow to underlying type |
| `DW_TAG_const_type` | Follow to underlying type |
| `DW_TAG_volatile_type` | Follow to underlying type |
| `DW_TAG_structure_type` | Recursive analysis |

---

## Binary Format Abstraction

### Supported Formats

| Format | Platform | DWARF Location |
|--------|----------|----------------|
| **ELF** | Linux, BSD | `.debug_*` sections |
| **Mach-O** | macOS, iOS | `__DWARF` segment |
| **PE/COFF** | Windows | `.debug_*` sections (with PDB) |

### The `object` Crate

```rust
use object::{Object, ObjectSection};

fn load_dwarf(path: &Path) -> Result<gimli::Dwarf<...>> {
    let data = std::fs::read(path)?;
    let object = object::File::parse(&data)?;

    // Unified interface across ELF/Mach-O/PE
    let debug_info = object.section_by_name(".debug_info")?;
    let debug_abbrev = object.section_by_name(".debug_abbrev")?;
    // ... etc
}
```

---

## gimli: The Foundation

### Why gimli?

| Feature | gimli | libdwarf (C) |
|---------|-------|--------------|
| **Memory Safety** | ✓ Guaranteed | ✗ Manual |
| **Zero-Copy Parsing** | ✓ References into buffer | ✗ Allocates per DIE |
| **Lazy Evaluation** | ✓ On-demand traversal | ✗ Full tree construction |
| **Modern C++ Support** | ✓ Robust | ✗ Crashes on lambdas |
| **Performance** | Excellent | Good (but leaks) |

### gimli Architecture

```
┌─────────────────────────────────────────────────────────┐
│                    Application                          │
├─────────────────────────────────────────────────────────┤
│  gimli::Dwarf<R>                                        │
│  ├── units() → Iterator<CompilationUnit>                │
│  ├── unit_ranges() → Address mappings                   │
│  └── supplementary debug info                           │
├─────────────────────────────────────────────────────────┤
│  gimli::read::*                                         │
│  ├── CompilationUnitHeader                              │
│  ├── DebuggingInformationEntry                          │
│  └── Attribute / AttributeValue                         │
├─────────────────────────────────────────────────────────┤
│  gimli::Reader trait                                    │
│  └── EndianSlice<'data, Endian>  (zero-copy!)           │
└─────────────────────────────────────────────────────────┘
```

### Key gimli Types

```rust
// Zero-copy reader over memory-mapped file
type R<'a> = gimli::EndianSlice<'a, gimli::LittleEndian>;

// Main entry point
let dwarf: gimli::Dwarf<R> = ...;

// Iterate compilation units
for unit in dwarf.units() {
    let unit = dwarf.unit(unit)?;

    // Iterate DIEs
    let mut entries = unit.entries();
    while let Some((_, entry)) = entries.next_dfs()? {
        if entry.tag() == gimli::DW_TAG_structure_type {
            // Found a struct!
        }
    }
}
```

---

## Performance Considerations

### Memory Mapping

Large binaries (games, kernels) can be multiple gigabytes. Memory-mapping avoids loading entire file into RAM:

```rust
use memmap2::Mmap;

let file = File::open(path)?;
let mmap = unsafe { Mmap::map(&file)? };
let object = object::File::parse(&mmap)?;
```

### Selective Parsing

struct-audit only needs type information, not:
- Line number tables (`.debug_line`)
- Call frame information (`.debug_frame`)
- Macro definitions (`.debug_macro`)

**Optimization**: Skip irrelevant sections entirely.

### Parallel CU Processing

Compilation Units are independent. For large binaries with thousands of CUs:

```rust
use rayon::prelude::*;

let units: Vec<_> = dwarf.units().collect();
let structs: Vec<_> = units.par_iter()
    .flat_map(|unit| extract_structs(unit))
    .collect();
```

---

## Edge Cases & Robustness

### 1. Missing Debug Info
- **Detection**: Check for empty `.debug_info`
- **Response**: Clear error message recommending `-g` flag

### 2. Split Debug Info
- **Formats**: `.dwo` files, `.dwp` packages, external debug directories
- **Solution**: Support `--debug-dir` flag for supplementary locations

### 3. Compressed Sections
- **Formats**: `.zdebug_*` (zlib), `SHF_COMPRESSED`
- **Solution**: Use `object` crate's decompression support

### 4. Rust Enums with Data
- **Challenge**: Discriminant layout varies by optimization
- **Solution**: Parse `DW_TAG_variant_part` for enum representations

### 5. C++ Templates
- **Challenge**: Each instantiation is a separate type
- **Solution**: Group by template name, show instantiation parameters

---

## Dependencies Summary

| Crate | Purpose | Version |
|-------|---------|---------|
| `gimli` | DWARF parsing | 0.28+ |
| `object` | Binary format abstraction | 0.32+ |
| `memmap2` | Memory-mapped file I/O | 0.9+ |
| `rayon` | Parallel processing | 1.8+ |

---

*Previous: [Market Analysis](./02-market.md) | Next: [Architecture](./04-architecture.md)*
