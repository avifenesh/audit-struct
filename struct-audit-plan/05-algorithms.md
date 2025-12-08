# Analysis Engine: Algorithms & Logic

## Executive Summary

This document specifies the core algorithms that power struct-audit's analysis capabilities: padding detection, cache efficiency analysis, bitfield handling, and optimization suggestions.

---

## Algorithm 1: Padding Detection

### Overview

Padding detection identifies "holes" in struct layouts—bytes inserted by the compiler for alignment but containing no useful data.

### Formal Definition

For a struct $S$ with members $M = \{m_1, m_2, ..., m_n\}$ sorted by offset:

$$\text{Padding}(m_i, m_{i+1}) = \text{Offset}(m_{i+1}) - (\text{Offset}(m_i) + \text{Size}(m_i))$$

Total internal padding:
$$\text{InternalPadding}(S) = \sum_{i=1}^{n-1} \max(0, \text{Padding}(m_i, m_{i+1}))$$

Tail padding:
$$\text{TailPadding}(S) = \text{Size}(S) - (\text{Offset}(m_n) + \text{Size}(m_n))$$

### Algorithm

```rust
fn detect_padding(struct_info: &StructInfo) -> PaddingReport {
    let mut members = struct_info.members.clone();
    members.sort_by_key(|m| m.offset);

    let mut holes = Vec::new();
    let mut total_padding = 0;

    // Check for leading padding (before first member)
    if !members.is_empty() && members[0].offset > 0 {
        holes.push(PaddingHole {
            offset: 0,
            size: members[0].offset,
            kind: PaddingKind::Leading,
        });
        total_padding += members[0].offset;
    }

    // Scan for internal gaps
    for window in members.windows(2) {
        let current = &window[0];
        let next = &window[1];

        let current_end = current.offset + current.size;
        let gap = next.offset.saturating_sub(current_end);

        if gap > 0 {
            holes.push(PaddingHole {
                offset: current_end,
                size: gap,
                kind: PaddingKind::Internal,
                after_member: Some(current.name.clone()),
            });
            total_padding += gap;
        }
    }

    // Check for tail padding
    if let Some(last) = members.last() {
        let last_end = last.offset + last.size;
        let tail = struct_info.total_size.saturating_sub(last_end);

        if tail > 0 {
            holes.push(PaddingHole {
                offset: last_end,
                size: tail,
                kind: PaddingKind::Tail,
            });
            total_padding += tail;
        }
    }

    PaddingReport {
        struct_name: struct_info.name.clone(),
        total_size: struct_info.total_size,
        data_bytes: struct_info.total_size - total_padding,
        padding_bytes: total_padding,
        holes,
        density: (struct_info.total_size - total_padding) as f64
                 / struct_info.total_size as f64,
    }
}
```

### Complexity

- **Time**: O(n log n) for sorting + O(n) for scanning = **O(n log n)**
- **Space**: O(n) for holes vector

---

## Algorithm 2: Cache Line Analysis

### Overview

Modern CPUs fetch memory in fixed-size blocks called **cache lines** (typically 64 bytes). Structs that span multiple cache lines or have members straddling boundaries incur performance penalties.

### Key Metrics

| Metric | Definition |
|--------|------------|
| **Cache Lines Spanned** | Number of 64-byte lines the struct occupies |
| **Straddling Members** | Members whose bytes cross a cache line boundary |
| **Cache Utilization** | Ratio of useful data to total cache lines fetched |

### Algorithm

```rust
const DEFAULT_CACHE_LINE_SIZE: usize = 64;

fn analyze_cache_efficiency(
    struct_info: &StructInfo,
    cache_line_size: usize,
) -> CacheAnalysis {
    let total_size = struct_info.total_size;

    // Calculate cache lines spanned
    let lines_spanned = (total_size + cache_line_size - 1) / cache_line_size;

    // Find members that straddle cache line boundaries
    let mut straddling = Vec::new();

    for member in &struct_info.members {
        let start_line = member.offset / cache_line_size;
        let end_offset = member.offset + member.size - 1;
        let end_line = end_offset / cache_line_size;

        if start_line != end_line {
            straddling.push(StraddlingMember {
                name: member.name.clone(),
                offset: member.offset,
                size: member.size,
                crosses_boundary_at: (start_line + 1) * cache_line_size,
            });
        }
    }

    // Calculate utilization
    let data_bytes = struct_info.members.iter().map(|m| m.size).sum::<usize>();
    let cache_bytes_fetched = lines_spanned * cache_line_size;
    let utilization = data_bytes as f64 / cache_bytes_fetched as f64;

    // Generate warnings
    let mut warnings = Vec::new();

    if lines_spanned > 1 && total_size <= cache_line_size + 8 {
        warnings.push(CacheWarning::NearlyFits {
            size: total_size,
            target: cache_line_size,
            excess: total_size - cache_line_size,
        });
    }

    if !straddling.is_empty() {
        warnings.push(CacheWarning::StraddlingMembers {
            count: straddling.len(),
        });
    }

    CacheAnalysis {
        cache_line_size,
        lines_spanned,
        straddling_members: straddling,
        utilization,
        warnings,
    }
}
```

### Visualization

```
Cache Line Analysis for `Order` (72 bytes, 2 cache lines)

    Cache Line 0 (bytes 0-63)        Cache Line 1 (bytes 64-127)
    ┌────────────────────────────┐   ┌────────────────────────────┐
    │ id     │timestamp│ price  │   │is_active│ PADDING (55 B)   │
    │ (8)    │  (8)    │  (8)   │   │  (1)    │                  │
    │        │         │        │   │         │                  │
    │ quantity│PADDING │ symbol │   │         │                  │
    │  (4)   │  (4)   │  (32)   │   │         │                  │
    └────────────────────────────┘   └────────────────────────────┘
    64 bytes                          8 bytes used / 64 available

    Utilization: 64/128 = 50% ⚠️
```

---

## Algorithm 3: Bitfield Packing

### Overview

Bitfields allow multiple values to share a single storage unit. Correct handling requires version-aware offset calculation.

### DWARF Version Detection

```rust
fn get_dwarf_version(cu: &CompilationUnitHeader) -> u16 {
    cu.version()
}
```

### Bitfield Offset Calculation

```rust
fn calculate_bitfield_offset(
    member: &DwarfMember,
    cu_version: u16,
    target_endian: Endianness,
) -> BitfieldInfo {
    let bit_size = member.bit_size.expect("bitfield must have bit_size");

    if cu_version >= 5 {
        // DWARF 5: data_bit_offset is straightforward
        let bit_offset = member.data_bit_offset
            .expect("DWARF 5 bitfield needs data_bit_offset");

        BitfieldInfo {
            bit_offset,
            bit_size,
            byte_offset: bit_offset / 8,
            bit_within_byte: bit_offset % 8,
        }
    } else {
        // DWARF 4 and earlier: bit_offset is relative to storage unit
        let raw_bit_offset = member.bit_offset
            .expect("DWARF 4 bitfield needs bit_offset");
        let storage_size = member.byte_size
            .expect("bitfield needs byte_size") * 8;

        let bit_offset = match target_endian {
            Endianness::Big => raw_bit_offset,
            Endianness::Little => {
                // Convert from big-endian-biased to actual offset
                storage_size - raw_bit_offset - bit_size
            }
        };

        // Add base offset from data_member_location
        let base_byte_offset = member.data_member_location.unwrap_or(0);
        let absolute_bit_offset = (base_byte_offset * 8) + bit_offset;

        BitfieldInfo {
            bit_offset: absolute_bit_offset,
            bit_size,
            byte_offset: absolute_bit_offset / 8,
            bit_within_byte: absolute_bit_offset % 8,
        }
    }
}
```

### Bitfield Visualization

```
struct Flags (4 bytes)
┌─────────────────────────────────────────────────────┐
│ Byte 0      │ Byte 1      │ Byte 2      │ Byte 3   │
├─────────────┼─────────────┼─────────────┼──────────┤
│ a:1 │ b:3   │ c:4 │ PADDING │              PADDING │
│ bit │ bits  │ bits│ (4 bits)│                      │
│ 0   │ 1-3   │ 4-7 │ 8-11    │ 12-31               │
└─────────────┴─────────────┴─────────────┴──────────┘

Bitfield Packing: 8 bits used / 32 available (25% utilization)
⚠️ Suggestion: Add more bitfields or reduce storage unit size
```

---

## Algorithm 4: Differential Analysis

### Overview

Comparing struct layouts between two versions (commits, branches, builds) to detect regressions.

### Identity Matching

Structs are matched by **fully qualified name**:
- `my_app::models::Order`
- `core_lib::Order` (different struct)

### Diff Categories

| Category | Description |
|----------|-------------|
| **Added** | Struct exists in head but not base |
| **Removed** | Struct exists in base but not head |
| **Changed** | Struct exists in both with different layout |
| **Unchanged** | Identical layout in both |

### Algorithm

```rust
fn diff_layouts(
    base: &LayoutReport,
    head: &LayoutReport,
) -> DiffReport {
    let base_map: HashMap<_, _> = base.structs.iter()
        .map(|s| (&s.name, s))
        .collect();

    let head_map: HashMap<_, _> = head.structs.iter()
        .map(|s| (&s.name, s))
        .collect();

    let mut added = Vec::new();
    let mut removed = Vec::new();
    let mut changed = Vec::new();
    let mut unchanged = Vec::new();

    // Find removed and changed
    for (name, base_struct) in &base_map {
        match head_map.get(name) {
            None => removed.push((*base_struct).clone()),
            Some(head_struct) => {
                if layouts_differ(base_struct, head_struct) {
                    changed.push(StructChange {
                        name: (*name).clone(),
                        base: (*base_struct).clone(),
                        head: (*head_struct).clone(),
                        delta: compute_delta(base_struct, head_struct),
                    });
                } else {
                    unchanged.push((*name).clone());
                }
            }
        }
    }

    // Find added
    for (name, head_struct) in &head_map {
        if !base_map.contains_key(name) {
            added.push((*head_struct).clone());
        }
    }

    DiffReport {
        base_commit: base.commit.clone(),
        head_commit: head.commit.clone(),
        added,
        removed,
        changed,
        unchanged_count: unchanged.len(),
        summary: compute_summary(&added, &removed, &changed),
    }
}

fn compute_delta(base: &StructInfo, head: &StructInfo) -> LayoutDelta {
    LayoutDelta {
        size_delta: head.total_size as i64 - base.total_size as i64,
        padding_delta: head.padding_bytes as i64 - base.padding_bytes as i64,
        member_delta: head.members.len() as i64 - base.members.len() as i64,
    }
}
```

### Change Classification

```rust
enum ChangeKind {
    SizeIncrease { from: usize, to: usize },
    SizeDecrease { from: usize, to: usize },
    PaddingIncrease { from: usize, to: usize },
    PaddingDecrease { from: usize, to: usize },
    MemberAdded { name: String },
    MemberRemoved { name: String },
    MemberReordered,
    MemberTypeChanged { name: String },
    CacheLineRegression { from: usize, to: usize },
}
```

---

## Algorithm 5: Optimization Suggestions

### Overview

Given a struct layout, suggest reorderings that minimize padding (a form of bin packing).

### Greedy Approach

Sort members by alignment requirement (descending), then by size (descending):

```rust
fn suggest_optimal_layout(struct_info: &StructInfo) -> OptimizationSuggestion {
    let mut members = struct_info.members.clone();

    // Sort by alignment (desc), then size (desc)
    members.sort_by(|a, b| {
        b.alignment.cmp(&a.alignment)
            .then_with(|| b.size.cmp(&a.size))
    });

    // Simulate packing
    let mut current_offset = 0;
    let mut new_layout = Vec::new();

    for member in &members {
        // Align to member's requirement
        let aligned_offset = align_up(current_offset, member.alignment);
        let padding = aligned_offset - current_offset;

        new_layout.push(OptimizedMember {
            name: member.name.clone(),
            offset: aligned_offset,
            size: member.size,
            original_offset: member.offset,
        });

        current_offset = aligned_offset + member.size;
    }

    // Calculate final size with struct alignment
    let struct_alignment = members.iter()
        .map(|m| m.alignment)
        .max()
        .unwrap_or(1);
    let final_size = align_up(current_offset, struct_alignment);

    let original_size = struct_info.total_size;
    let savings = original_size.saturating_sub(final_size);

    OptimizationSuggestion {
        original_size,
        optimized_size: final_size,
        savings,
        savings_percent: (savings as f64 / original_size as f64) * 100.0,
        suggested_order: new_layout,
        applicable: savings > 0,
    }
}

fn align_up(offset: usize, alignment: usize) -> usize {
    (offset + alignment - 1) & !(alignment - 1)
}
```

### Limitations

1. **Language Constraints**: Rust tuple structs and C designated initializers may depend on field order
2. **ABI Stability**: Reordering may break serialization or FFI
3. **Cache Locality**: Sometimes "hot" fields should be grouped regardless of padding

### Output Example

```
Optimization Suggestion for `Order`:

Current Layout (72 bytes):
  0: id (u64, 8 bytes)
  8: timestamp (i64, 8 bytes)
 16: price (f64, 8 bytes)
 24: quantity (u32, 4 bytes)
 28: [PADDING, 4 bytes]
 32: symbol ([u8; 32], 32 bytes)
 64: is_active (bool, 1 byte)
 65: [PADDING, 7 bytes]

Suggested Layout (68 bytes, -4 bytes, -5.5%):
  0: id (u64, 8 bytes)
  8: timestamp (i64, 8 bytes)
 16: price (f64, 8 bytes)
 24: symbol ([u8; 32], 32 bytes)
 56: quantity (u32, 4 bytes)
 60: is_active (bool, 1 byte)
 61: [PADDING, 3 bytes]           ◄ Reduced from 11 bytes total

Note: Reordering may affect ABI compatibility. Review before applying.
```

---

## Algorithm 6: Type Resolution

### Overview

DWARF stores types as references. Resolving to concrete size requires following the reference chain.

### Resolution Chain

```
DW_AT_type → typedef → const → pointer → base_type
                                              │
                                         byte_size=8
```

### Algorithm

```rust
fn resolve_type_size(
    dwarf: &Dwarf,
    unit: &CompilationUnit,
    type_offset: UnitOffset,
) -> Result<TypeInfo> {
    let mut current = type_offset;
    let mut modifiers = Vec::new();

    loop {
        let entry = unit.entry(current)?;

        match entry.tag() {
            // Terminal types (have byte_size)
            DW_TAG_base_type |
            DW_TAG_structure_type |
            DW_TAG_union_type |
            DW_TAG_enumeration_type |
            DW_TAG_class_type => {
                let size = entry.attr(DW_AT_byte_size)?
                    .ok_or("type missing byte_size")?;
                let name = entry.attr(DW_AT_name)?
                    .map(|a| dwarf.attr_string(&unit, a));

                return Ok(TypeInfo {
                    size,
                    name,
                    modifiers,
                    is_primitive: entry.tag() == DW_TAG_base_type,
                });
            }

            // Pointer types
            DW_TAG_pointer_type |
            DW_TAG_reference_type |
            DW_TAG_rvalue_reference_type => {
                modifiers.push(TypeModifier::Pointer);
                // Pointer size is architecture-dependent
                return Ok(TypeInfo {
                    size: std::mem::size_of::<*const ()>(),
                    name: Some("*".into()),
                    modifiers,
                    is_primitive: false,
                });
            }

            // Array types
            DW_TAG_array_type => {
                let element_type = entry.attr(DW_AT_type)?
                    .ok_or("array missing element type")?;
                let element_info = resolve_type_size(dwarf, unit, element_type)?;

                // Get array bounds
                let count = get_array_count(&entry)?;

                return Ok(TypeInfo {
                    size: element_info.size * count,
                    name: Some(format!("[{}; {}]", element_info.name.unwrap_or_default(), count)),
                    modifiers,
                    is_primitive: false,
                });
            }

            // Pass-through modifiers
            DW_TAG_typedef => {
                modifiers.push(TypeModifier::Typedef(
                    entry.attr(DW_AT_name).map(|a| dwarf.attr_string(&unit, a))
                ));
            }
            DW_TAG_const_type => modifiers.push(TypeModifier::Const),
            DW_TAG_volatile_type => modifiers.push(TypeModifier::Volatile),
            DW_TAG_restrict_type => modifiers.push(TypeModifier::Restrict),
            DW_TAG_atomic_type => modifiers.push(TypeModifier::Atomic),

            _ => return Err(format!("unexpected type tag: {:?}", entry.tag())),
        }

        // Follow to underlying type
        current = entry.attr(DW_AT_type)?
            .ok_or("modifier type missing DW_AT_type")?
            .as_unit_offset()?;
    }
}
```

---

## Metrics Summary

| Metric | Formula | Ideal |
|--------|---------|-------|
| **Density** | data_bytes / total_size | 1.0 (100%) |
| **Cache Utilization** | data_bytes / (cache_lines × 64) | 1.0 (100%) |
| **Padding Ratio** | padding_bytes / total_size | 0.0 (0%) |
| **Cache Fit** | total_size ≤ 64 ? true : false | true |

---

*Previous: [Architecture](./04-architecture.md) | Next: [Business](./06-business.md)*
