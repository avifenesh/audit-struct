# Analysis Algorithms

## Padding Detection, Cache Analysis, and Optimization

---

## 1. Padding Detection Algorithm

### 1.1 Overview

The core task is identifying **implicit padding** inserted by the compiler for alignment.

**Natural Alignment Rule**: A data type of size N is usually aligned to an address divisible by N.

### 1.2 Algorithm Steps

```
┌─────────────────────────────────────────────────────────────────┐
│                  Padding Detection Pipeline                      │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│  1. COLLECT                                                      │
│     └── Gather all DW_TAG_member children of struct              │
│                                                                  │
│  2. RESOLVE                                                      │
│     └── Resolve byte size of each member's type                  │
│                                                                  │
│  3. SORT                                                         │
│     └── Sort members by DW_AT_data_member_location               │
│                                                                  │
│  4. SCAN                                                         │
│     └── Iterate and detect gaps between members                  │
│                                                                  │
│  5. TAIL                                                         │
│     └── Check for tail padding after last member                 │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

### 1.3 Formal Definition

Let \( M \) be the sorted list of members.

For each member \( M_i \):
- \( Offset(M_i) \) = start byte of member \( i \)
- \( Size(M_i) \) = size in bytes of member \( i \)

**Gap Detection**:
```
End(Mᵢ) = Offset(Mᵢ) + Size(Mᵢ)
Gap = Offset(Mᵢ₊₁) - End(Mᵢ)

If Gap > 0 → Record PADDING HOLE of size Gap at address End(Mᵢ)
```

**Tail Padding**:
```
TailPad = StructSize - End(M_last)

If TailPad > 0 → Record TAIL PADDING
```

### 1.4 Implementation

```rust
pub fn detect_padding(struct_layout: &StructLayout) -> Vec<PaddingHole> {
    let mut holes = Vec::new();
    let members = &struct_layout.members;
    
    if members.is_empty() {
        return holes;
    }
    
    // Sort members by offset
    let mut sorted: Vec<_> = members.iter().collect();
    sorted.sort_by_key(|m| m.offset);
    
    // Scan for gaps between members
    for window in sorted.windows(2) {
        let current = window[0];
        let next = window[1];
        
        let end_of_current = current.offset + current.size;
        let gap = next.offset.saturating_sub(end_of_current);
        
        if gap > 0 {
            holes.push(PaddingHole {
                offset: end_of_current,
                size: gap,
                after_field: current.name.clone(),
                before_field: next.name.clone(),
            });
        }
    }
    
    // Check for tail padding
    if let Some(last) = sorted.last() {
        let end_of_last = last.offset + last.size;
        let tail_pad = struct_layout.size.saturating_sub(end_of_last);
        
        if tail_pad > 0 {
            holes.push(PaddingHole {
                offset: end_of_last,
                size: tail_pad,
                after_field: last.name.clone(),
                before_field: "<end>".to_string(),
            });
        }
    }
    
    holes
}
```

### 1.5 Example

```c
struct Order {
    uint64_t id;        // offset 0,  size 8
    bool is_active;     // offset 8,  size 1
    // [7 bytes padding]
    double price;       // offset 16, size 8
    bool is_filled;     // offset 24, size 1
    // [7 bytes padding]
};  // Total: 32 bytes
```

**Detected Holes**:
| Offset | Size | After Field | Before Field |
|--------|------|-------------|--------------|
| 9 | 7 | is_active | price |
| 25 | 7 | is_filled | \<end\> |

---

## 2. Cache Line Analysis

### 2.1 Cache Line Basics

- **Cache Line Size**: Typically 64 bytes (x86/ARM)
- **Goal**: Maximize useful data per cache line fetch

### 2.2 Straddle Detection

A member **straddles** a cache line if it spans two cache lines:

```
StartLine = floor(Offset(Mᵢ) / 64)
EndLine = floor((Offset(Mᵢ) + Size(Mᵢ) - 1) / 64)

If StartLine ≠ EndLine → STRADDLE VIOLATION
```

### 2.3 Implementation

```rust
pub struct CacheLineAnalysis {
    pub cache_line_size: u64,
    pub lines_spanned: u64,
    pub density: f64,
    pub violations: Vec<CacheLineViolation>,
}

pub fn analyze_cache_lines(
    struct_layout: &StructLayout,
    cache_line_size: u64,
) -> CacheLineAnalysis {
    let mut violations = Vec::new();
    
    // Check each member for straddling
    for member in &struct_layout.members {
        let start_line = member.offset / cache_line_size;
        let end_byte = member.offset + member.size - 1;
        let end_line = end_byte / cache_line_size;
        
        if start_line != end_line {
            violations.push(CacheLineViolation {
                field_name: member.name.clone(),
                offset: member.offset,
                size: member.size,
                lines: vec![start_line, end_line],
            });
        }
    }
    
    // Calculate total cache lines spanned by struct
    let lines_spanned = (struct_layout.size + cache_line_size - 1) 
                        / cache_line_size;
    
    // Calculate data density
    let useful_bytes: u64 = struct_layout.members
        .iter()
        .map(|m| m.size)
        .sum();
    
    let total_cache_bytes = lines_spanned * cache_line_size;
    let density = useful_bytes as f64 / total_cache_bytes as f64;
    
    CacheLineAnalysis {
        cache_line_size,
        lines_spanned,
        density,
        violations,
    }
}
```

### 2.4 Density Metric

```
Density = Σ Size(Members) / (Total Cache Lines × 64)
```

| Density | Rating | Interpretation |
|---------|--------|----------------|
| > 0.9 | Excellent | Minimal waste |
| 0.7-0.9 | Good | Acceptable |
| 0.5-0.7 | Fair | Room for improvement |
| < 0.5 | Poor | Significant waste |

---

## 3. Bitfield Packing Analysis

### 3.1 Bitfield Tracking

When analyzing bitfields, track the "current bit cursor":

```rust
pub struct BitfieldContext {
    pub storage_offset: u64,      // Byte offset of storage unit
    pub storage_size: u64,        // Size of storage unit in bytes
    pub current_bit: u64,         // Current bit position within unit
    pub fields: Vec<BitfieldMember>,
}

pub fn analyze_bitfields(members: &[MemberLayout]) -> Vec<BitfieldContext> {
    let mut contexts = Vec::new();
    let mut current_context: Option<BitfieldContext> = None;
    
    for member in members {
        if let (Some(bit_offset), Some(bit_size)) = 
            (member.bit_offset, member.bit_size) 
        {
            // This is a bitfield
            match &mut current_context {
                Some(ctx) if ctx.storage_offset == member.offset => {
                    // Same storage unit
                    ctx.fields.push(BitfieldMember {
                        name: member.name.clone(),
                        bit_offset,
                        bit_size,
                    });
                    ctx.current_bit = bit_offset + bit_size;
                }
                _ => {
                    // New storage unit
                    if let Some(ctx) = current_context.take() {
                        contexts.push(ctx);
                    }
                    current_context = Some(BitfieldContext {
                        storage_offset: member.offset,
                        storage_size: member.size,
                        current_bit: bit_offset + bit_size,
                        fields: vec![BitfieldMember {
                            name: member.name.clone(),
                            bit_offset,
                            bit_size,
                        }],
                    });
                }
            }
        } else {
            // Non-bitfield, close current context
            if let Some(ctx) = current_context.take() {
                contexts.push(ctx);
            }
        }
    }
    
    if let Some(ctx) = current_context {
        contexts.push(ctx);
    }
    
    contexts
}
```

### 3.2 Optimization Opportunity

Detect when `bool` fields could be merged into bitfields:

```rust
pub fn suggest_bitfield_optimization(
    members: &[MemberLayout]
) -> Vec<OptimizationSuggestion> {
    let mut suggestions = Vec::new();
    
    // Find sequences of bool fields
    let bool_sequences: Vec<_> = members
        .iter()
        .enumerate()
        .filter(|(_, m)| m.type_name == "bool" && m.bit_size.is_none())
        .collect();
    
    if bool_sequences.len() >= 2 {
        let potential_savings = (bool_sequences.len() - 1) as u64;
        suggestions.push(OptimizationSuggestion {
            kind: SuggestionKind::MergeBoolsToBitfield,
            fields: bool_sequences.iter()
                .map(|(_, m)| m.name.clone())
                .collect(),
            savings_bytes: potential_savings,
            description: format!(
                "Merge {} bool fields into bitfield to save {} bytes",
                bool_sequences.len(),
                potential_savings
            ),
        });
    }
    
    suggestions
}
```

---

## 4. Optimal Layout Algorithm

### 4.1 The Bin Packing Problem

Finding the optimal field order is a variant of the **bin packing problem**.

**Goal**: Minimize total struct size by reordering fields.

### 4.2 Greedy Algorithm

Sort fields by alignment requirement (descending), then by size (descending):

```rust
pub fn compute_optimal_layout(
    members: &[MemberLayout]
) -> Vec<MemberLayout> {
    let mut sorted = members.to_vec();
    
    // Sort by alignment (desc), then size (desc)
    sorted.sort_by(|a, b| {
        b.alignment.cmp(&a.alignment)
            .then_with(|| b.size.cmp(&a.size))
    });
    
    sorted
}

pub fn calculate_packed_size(members: &[MemberLayout]) -> u64 {
    let mut offset: u64 = 0;
    let mut max_alignment: u64 = 1;
    
    for member in members {
        // Align offset for this member
        let alignment = member.alignment;
        offset = (offset + alignment - 1) & !(alignment - 1);
        
        // Add member size
        offset += member.size;
        
        // Track max alignment for tail padding
        max_alignment = max_alignment.max(alignment);
    }
    
    // Add tail padding for array alignment
    (offset + max_alignment - 1) & !(max_alignment - 1)
}
```

### 4.3 Example Optimization

**Before** (developer order):
```c
struct Order {
    bool is_active;     // 1 byte, align 1
    uint64_t id;        // 8 bytes, align 8 → 7 bytes padding before
    bool is_filled;     // 1 byte, align 1
    double price;       // 8 bytes, align 8 → 7 bytes padding before
};  // Size: 32 bytes, Padding: 14 bytes
```

**After** (optimal order):
```c
struct Order {
    uint64_t id;        // 8 bytes, align 8
    double price;       // 8 bytes, align 8
    bool is_active;     // 1 byte, align 1
    bool is_filled;     // 1 byte, align 1
    // [6 bytes tail padding for array alignment]
};  // Size: 24 bytes, Padding: 6 bytes
```

**Savings**: 8 bytes per instance (25%)

---

## 5. Diff Algorithm

### 5.1 Struct Matching

Match structs between two binaries by fully-qualified name:

```rust
pub fn match_structs(
    old: &[StructLayout],
    new: &[StructLayout],
) -> StructDiff {
    let old_map: HashMap<_, _> = old.iter()
        .map(|s| (&s.name, s))
        .collect();
    
    let new_map: HashMap<_, _> = new.iter()
        .map(|s| (&s.name, s))
        .collect();
    
    let mut added = Vec::new();
    let mut removed = Vec::new();
    let mut changed = Vec::new();
    
    // Find removed and changed
    for (name, old_struct) in &old_map {
        match new_map.get(name) {
            Some(new_struct) => {
                if !layouts_equal(old_struct, new_struct) {
                    changed.push(StructChange {
                        name: name.to_string(),
                        old: (*old_struct).clone(),
                        new: (*new_struct).clone(),
                    });
                }
            }
            None => removed.push((*old_struct).clone()),
        }
    }
    
    // Find added
    for (name, new_struct) in &new_map {
        if !old_map.contains_key(name) {
            added.push((*new_struct).clone());
        }
    }
    
    StructDiff { added, removed, changed }
}
```

### 5.2 Change Classification

```rust
pub enum ChangeType {
    SizeIncrease { old: u64, new: u64 },
    SizeDecrease { old: u64, new: u64 },
    PaddingIncrease { old: u64, new: u64 },
    PaddingDecrease { old: u64, new: u64 },
    CacheLineIncrease { old: u64, new: u64 },
    FieldAdded { name: String },
    FieldRemoved { name: String },
    FieldReordered,
}

pub fn classify_changes(change: &StructChange) -> Vec<ChangeType> {
    let mut types = Vec::new();
    
    // Size changes
    if change.new.size > change.old.size {
        types.push(ChangeType::SizeIncrease {
            old: change.old.size,
            new: change.new.size,
        });
    } else if change.new.size < change.old.size {
        types.push(ChangeType::SizeDecrease {
            old: change.old.size,
            new: change.new.size,
        });
    }
    
    // Padding changes
    let old_pad = change.old.metrics.padding_bytes;
    let new_pad = change.new.metrics.padding_bytes;
    
    if new_pad > old_pad {
        types.push(ChangeType::PaddingIncrease {
            old: old_pad,
            new: new_pad,
        });
    }
    
    // Cache line changes (critical!)
    if change.new.metrics.cache_lines > change.old.metrics.cache_lines {
        types.push(ChangeType::CacheLineIncrease {
            old: change.old.metrics.cache_lines,
            new: change.new.metrics.cache_lines,
        });
    }
    
    types
}
```

---

## 6. Budget Evaluation

### 6.1 Budget Rules

```rust
pub struct Budget {
    pub pattern: String,  // Exact name or glob
    pub max_size: Option<u64>,
    pub max_padding_percent: Option<f64>,
    pub max_cache_lines: Option<u64>,
}

pub struct BudgetViolation {
    pub struct_name: String,
    pub budget: Budget,
    pub violations: Vec<ViolationType>,
}

pub enum ViolationType {
    SizeExceeded { actual: u64, max: u64 },
    PaddingExceeded { actual: f64, max: f64 },
    CacheLinesExceeded { actual: u64, max: u64 },
}
```

### 6.2 Evaluation

```rust
pub fn evaluate_budgets(
    structs: &[StructLayout],
    budgets: &[Budget],
) -> Vec<BudgetViolation> {
    let mut violations = Vec::new();
    
    for struct_layout in structs {
        for budget in budgets {
            if matches_pattern(&struct_layout.name, &budget.pattern) {
                let struct_violations = check_budget(struct_layout, budget);
                
                if !struct_violations.is_empty() {
                    violations.push(BudgetViolation {
                        struct_name: struct_layout.name.clone(),
                        budget: budget.clone(),
                        violations: struct_violations,
                    });
                }
            }
        }
    }
    
    violations
}

fn check_budget(
    s: &StructLayout, 
    b: &Budget
) -> Vec<ViolationType> {
    let mut v = Vec::new();
    
    if let Some(max) = b.max_size {
        if s.size > max {
            v.push(ViolationType::SizeExceeded { 
                actual: s.size, max 
            });
        }
    }
    
    if let Some(max) = b.max_padding_percent {
        if s.metrics.padding_percent > max {
            v.push(ViolationType::PaddingExceeded { 
                actual: s.metrics.padding_percent, max 
            });
        }
    }
    
    if let Some(max) = b.max_cache_lines {
        if s.metrics.cache_lines > max {
            v.push(ViolationType::CacheLinesExceeded { 
                actual: s.metrics.cache_lines, max 
            });
        }
    }
    
    v
}
```

---

## Next Steps

→ [Business Model](./07-business-model.md) - Pricing and go-to-market strategy


