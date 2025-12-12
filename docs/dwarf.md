# DWARF Parsing Reference

## Overview

DWARF (Debugging With Attributed Record Formats) is the debug information format used by GCC, Clang, and rustc. struct-audit parses DWARF to extract struct layouts.

## DWARF Structure

```
.debug_info     DIEs (Debugging Information Entries)
.debug_abbrev   Abbreviation tables (DIE templates)
.debug_str      String table
.debug_line     Source line mapping
```

## Key Tags

| Tag | Description |
|-----|-------------|
| `DW_TAG_structure_type` | C/C++/Rust struct |
| `DW_TAG_class_type` | C++ class |
| `DW_TAG_member` | Struct field |
| `DW_TAG_inheritance` | C++ base class |
| `DW_TAG_base_type` | Primitive type |
| `DW_TAG_pointer_type` | Pointer |
| `DW_TAG_array_type` | Array |
| `DW_TAG_typedef` | Type alias |

## Key Attributes

| Attribute | Description |
|-----------|-------------|
| `DW_AT_name` | Type/member name |
| `DW_AT_byte_size` | Size in bytes |
| `DW_AT_alignment` | Alignment requirement |
| `DW_AT_data_member_location` | Member offset |
| `DW_AT_type` | Reference to type DIE |
| `DW_AT_bit_size` | Bitfield size |
| `DW_AT_bit_offset` | DWARF 4 bitfield offset |
| `DW_AT_data_bit_offset` | DWARF 5 bitfield offset |

## Member Offset Resolution

`DW_AT_data_member_location` can be:

1. **Constant integer**: Direct byte offset (most common)
2. **DWARF expression**: Stack-based bytecode for complex layouts

```rust
// Simple case: constant
AttributeValue::Udata(offset) => offset

// Complex case: expression (C++ virtual inheritance)
AttributeValue::Exprloc(expr) => evaluate(expr)
```

### Expression Evaluation

Common operations:
- `DW_OP_plus_uconst N`: Add N to stack top
- `DW_OP_constu N`: Push constant N

For virtual inheritance, expressions may require runtime values (`DW_OP_deref`). These are marked as unknown offset.

## Bitfield Handling

### DWARF 4

Uses `DW_AT_bit_offset` (big-endian biased):

```rust
// Convert DWARF 4 bit_offset to actual position (little-endian)
let actual_bit_offset = storage_bits - raw_bit_offset - bit_size;
```

### DWARF 5

Uses `DW_AT_data_bit_offset` (endian-independent):

```rust
// Direct offset from struct start in bits
let bit_offset = data_bit_offset;
let byte_offset = data_bit_offset / 8;
```

### Implementation

```rust
if let Some(bit_size) = bit_size {
    if let Some(data_bit_offset) = dwarf5_data_bit_offset {
        // DWARF 5: direct
        member.bit_offset = Some(data_bit_offset % 8);
    } else if let Some(raw_bit_offset) = dwarf4_bit_offset {
        // DWARF 4: convert based on endianness
        let bit_offset = match endian {
            LittleEndian => storage_bits - raw_bit_offset - bit_size,
            BigEndian => raw_bit_offset,
        };
        member.bit_offset = Some(bit_offset);
    }
}
```

## Type Resolution

Types form chains that must be followed:

```
DW_TAG_member "id"
  └─► DW_AT_type ─► DW_TAG_typedef "OrderId"
                      └─► DW_AT_type ─► DW_TAG_base_type "u64"
                                          └─► DW_AT_byte_size = 8
```

Implementation uses caching to avoid repeated traversal:

```rust
pub struct TypeResolver<'a, 'b> {
    cache: HashMap<UnitOffset, (String, Option<u64>)>,
    // ...
}

fn resolve_type(&mut self, offset: UnitOffset) -> Result<(String, Option<u64>)> {
    if let Some(cached) = self.cache.get(&offset) {
        return Ok(cached.clone());
    }
    let result = self.resolve_type_inner(offset, 0)?;
    self.cache.insert(offset, result.clone());
    Ok(result)
}
```

## Edge Cases

### Anonymous Structs
No `DW_AT_name` attribute. Generated name: `<anonymous>`.

### Packed Structs
`DW_AT_byte_size` won't match aligned size. No padding inserted.

### Zero-Sized Types (Rust)
`DW_AT_byte_size = 0`. Valid, reported as-is.

### Incomplete Types
Forward declarations lack `DW_AT_byte_size`. Skipped.

### Compressed Sections
`.zdebug_*` sections are zlib-compressed. Decompressed on load.

## gimli Usage

```rust
use gimli::{Dwarf, EndianSlice, RunTimeEndian};

// Load DWARF sections
let dwarf = Dwarf::load(|section| {
    let data = object.section_by_name(section.name())
        .map(|s| s.data().unwrap_or(&[]))
        .unwrap_or(&[]);
    Ok(EndianSlice::new(data, endian))
})?;

// Iterate compilation units
let mut units = dwarf.units();
while let Some(header) = units.next()? {
    let unit = dwarf.unit(header)?;
    // Process DIEs...
}
```
