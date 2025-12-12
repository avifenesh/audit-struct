# Analysis Algorithms

## Padding Detection

### Overview

Detects implicit padding bytes inserted by the compiler for alignment.

### Algorithm

1. Collect members with known offsets and sizes
2. Sort by offset
3. Merge overlapping spans (bitfields, unions)
4. Detect gaps between spans
5. Detect tail padding

```
Input: Members with (offset, size)
Output: List of PaddingHole

sorted_spans = sort_by_offset(members)
merged_spans = merge_overlapping(sorted_spans)

for each consecutive pair (span_a, span_b):
    gap = span_b.start - span_a.end
    if gap > 0:
        emit PaddingHole(offset=span_a.end, size=gap)

tail = struct_size - last_span.end
if tail > 0:
    emit PaddingHole(offset=last_span.end, size=tail)
```

### Span Merging

Handles overlapping members (bitfields share storage units, unions overlap):

```rust
let mut current_start = spans[0].start;
let mut current_end = spans[0].end;

for span in spans.skip(1) {
    if span.start > current_end {
        // Gap found - emit merged span, start new
        useful_size += current_end - current_start;
        emit_hole_if_gap(current_end, span.start);
        current_start = span.start;
        current_end = span.end;
    } else if span.end > current_end {
        // Overlapping - extend current span
        current_end = span.end;
    }
}
```

### Partial Analysis

When member offsets are unknown (complex C++ inheritance), `partial=true` is set and padding holes are not reported (would be inaccurate).

## Cache Line Analysis

### Metrics

- **cache_lines_spanned**: `ceil(struct_size / cache_line_size)`
- **cache_line_density**: `useful_size / (lines * line_size) * 100`

```rust
let cache_line_size_u64 = cache_line_size as u64;
let cache_lines_spanned = if layout.size > 0 {
    layout.size.div_ceil(cache_line_size_u64) as u32
} else {
    0
};

let cache_line_density = if cache_lines_spanned > 0 {
    let total_cache_bytes = cache_lines_spanned as u64 * cache_line_size_u64;
    (useful_size as f64 / total_cache_bytes as f64) * 100.0
} else {
    0.0
};
```

### Density Interpretation

| Density | Rating |
|---------|--------|
| > 90% | Excellent |
| 70-90% | Good |
| 50-70% | Fair |
| < 50% | Poor |

## Diff Algorithm

### Struct Matching

Match structs by fully-qualified name between two binaries:

```rust
let old_map: HashMap<&str, &StructLayout> = old.iter()
    .map(|s| (s.name.as_str(), s))
    .collect();

let new_map: HashMap<&str, &StructLayout> = new.iter()
    .map(|s| (s.name.as_str(), s))
    .collect();

// Removed: in old, not in new
// Added: in new, not in old
// Changed: in both, but different
```

### Change Detection

```rust
pub struct StructChange {
    pub name: String,
    pub old_size: u64,
    pub new_size: u64,
    pub size_delta: i64,
    pub old_padding: u64,
    pub new_padding: u64,
    pub padding_delta: i64,
    pub member_changes: Vec<MemberChange>,
}

pub fn has_regressions(&self) -> bool {
    self.changed.iter().any(|c| c.size_delta > 0 || c.padding_delta > 0)
}
```

### Member Diff

For changed structs, detect:
- Added members
- Removed members
- Offset changes
- Size changes
- Type changes

## Budget Evaluation

### Config Format

```yaml
budgets:
  StructName:
    max_size: 64
    max_padding: 8
    max_padding_percent: 15.0
```

### Validation

```rust
for (name, budget) in &config.budgets {
    if let Some(layout) = layouts.iter().find(|l| l.name == *name) {
        if let Some(max) = budget.max_size {
            if layout.size > max {
                violations.push(...);
            }
        }
        if let Some(max) = budget.max_padding {
            if layout.metrics.padding_bytes > max {
                violations.push(...);
            }
        }
        if let Some(max) = budget.max_padding_percent {
            if layout.metrics.padding_percentage > max {
                violations.push(...);
            }
        }
    }
}
```

Exit code 1 if any violations found.

## Complexity

| Algorithm | Time | Space |
|-----------|------|-------|
| Padding detection | O(n log n) | O(n) |
| Cache analysis | O(1) | O(1) |
| Diff | O(n + m) | O(n + m) |
| Budget check | O(s × b) | O(v) |

Where n=members, m=new structs, s=structs, b=budgets, v=violations.
