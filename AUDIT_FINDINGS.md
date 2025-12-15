# layout-audit code audit: findings and fixes (Dec 15, 2025)

This file captures the issues found during a deep review + the changes made to fix them, with concrete repro steps and how each was validated. It is intentionally evidence-driven and avoids speculative "nice-to-haves".

## Commit status

| Finding | Description | Commit |
|---------|-------------|--------|
| 1 + 2 | Deterministic diff output + struct deduplication | 22215a7 |
| 3 | --max-align validation | c550eae |
| 4 | GitHub Action docs | e0d9189 |
| 5 | diff.rs: overflow handling, docs, optimization | 8c45734 |
| 6 | context.rs: stable dedup, helper consolidation, overflow | 4cdaaad |
| 7 | optimize.rs: overflow protection, test coverage | a51e7ca |
| 8 | optimize.rs: bitfield grouping, bit_offset, zero-size | f1e01cf |
| 9 | diff.rs: scoring constants, BTreeMap, count_delta | e2acfb3 |
| 10 | context.rs: type resolution helper, checked arithmetic | 8151a56 |
| 11 | types.rs: checked_sub for cross-unit refs | e5e8b99 |
| 12 | optimize.rs: bitfield tracking, no silent data loss | 6d1d017 |
| 13 | context.rs: consolidate attribute parsing | e88d7f3 |
| 14-16 | Fifth pass: overflow, clone, code consolidation | 85214cf, 688b9bb |

## Environment and validation

### Commands run

- `cargo fmt --check` (and `cargo fmt` after changes)
- `cargo test`
- `cargo clippy --all-targets --all-features -- -D warnings`
- Repeated CLI runs against:
  - `tests/fixtures/bin/test_simple.dSYM/Contents/Resources/DWARF/test_simple`
  - `tests/fixtures/bin/test_modified.dSYM/Contents/Resources/DWARF/test_modified`
  - `tests/fixtures/bin/test_cpp_templates.dSYM/Contents/Resources/DWARF/test_cpp_templates`
  - `tests/fixtures/rust_test/target/debug/rust_test_structs.dSYM/Contents/Resources/DWARF/rust_test_structs`
  - Two small locally compiled Rust repro binaries in `/tmp` (see below)

### Result summary

- All tests + clippy warnings are clean after the fixes.
- The previously demonstrated nondeterminism in `diff -o json` is eliminated.
- Duplicate identical struct entries are eliminated (prevents double-counting in `check` and duplicate `inspect` output).
- `suggest --max-align 0` is rejected at the CLI level; internal alignment math is now correct for non-power-of-two values as well.

## Finding 1: `diff` JSON output was non-deterministic

### Symptom (before)

Running the same command multiple times produced different JSON byte-for-byte (the data was the same but ordering was not stable). This breaks any consumer that treats the JSON text as stable (CI snapshots, log diffing, caching, etc.).

### Repro (before)

On macOS with dSYM fixtures:

```sh
cargo run --quiet -- \
  diff tests/fixtures/bin/test_simple.dSYM/Contents/Resources/DWARF/test_simple \
       tests/fixtures/bin/test_modified.dSYM/Contents/Resources/DWARF/test_modified \
       -o json \
  > /tmp/layout_audit_diff.json
shasum -a 256 /tmp/layout_audit_diff.json
```

Repeating the command and hashing the output yielded multiple different digests. A concrete observed `diff -u` showed swapped ordering inside `changed[].member_changes` (e.g., two `OffsetChanged` entries swapping order).

### Root cause (before)

- `member_changes` was built by iterating `HashMap`s; iteration order is randomized per process.
- `member_changes` was never sorted before printing/serialization.

### Fix

Implemented stable ordering and stable matching:

- `member_changes` is sorted deterministically by (kind rank, name, details).
- `diff_layouts()` no longer collapses structs into a single `HashMap<&str, &StructLayout>`; it now groups by name and matches duplicates deterministically.

Code: `src/diff.rs`.

### Validation (after)

Repeated runs now produce identical JSON:

```sh
for i in $(seq 1 10); do
  cargo run --quiet -- diff <OLD> <NEW> -o json > /tmp/d.json
  shasum -a 256 /tmp/d.json | awk '{print $1}'
done | sort | uniq -c
```

Result: a single unique hash count.

## Finding 2: Struct name collisions and duplicate entries caused incorrect diffs/checks

### Symptom (before)

The system used `DW_AT_name` as the primary identity. In practice:

- `inspect` could output multiple distinct layouts with the same displayed `name`.
- `diff` keyed structs by `name`, causing collisions: for duplicates, one would overwrite another in the map and diffs could compare the “wrong” pair.
- `check` could double-count identical duplicates and produce duplicated violations.

### Repro A: same-name distinct Rust structs

Compile:

```sh
cat > /tmp/layout_audit_dup_rust.rs <<'RS'
#![allow(dead_code)]

mod a {
    #[repr(C)]
    pub struct Foo {
        pub x: u64,
    }
}

mod b {
    #[repr(C)]
    pub struct Foo {
        pub x: u32,
        pub y: u32,
    }
}

fn main() {
    let a = a::Foo { x: 1 };
    let b = b::Foo { x: 2, y: 3 };
    std::hint::black_box(a);
    std::hint::black_box(b);
}
RS
rustc -g /tmp/layout_audit_dup_rust.rs -o /tmp/layout_audit_dup_rust_bin
```

Inspect:

```sh
cargo run --quiet -- \
  inspect /tmp/layout_audit_dup_rust_bin.dSYM/Contents/Resources/DWARF/layout_audit_dup_rust_bin \
  --filter Foo -o json
```

Observed: two structs both named `Foo`, but with different members.

### Fixes

Two targeted changes:

1) Deduplicate exact duplicate struct entries (but **not** distinct same-name types):
   - `DwarfContext::find_structs()` now deduplicates **exact duplicates** by a fingerprint of:
     `name`, `size`, `alignment`, `source_location`, and the full member list (including offsets, sizes, bitfields, atomic flag).
   - This prevents printing the same struct twice and avoids double-counting in budget checks.
   - Code: `src/dwarf/context.rs`.

2) Make `diff` robust to same-name duplicates:
   - `diff_layouts()` now groups by name and matches duplicates deterministically rather than overwriting them in a map.
   - Matching strategy:
     - first pair exact matches by `(source_location.file, source_location.line)` when present
     - then greedy matching by a deterministic similarity score based on member overlap and size/padding proximity
   - Code: `src/diff.rs`.

### Validation (after)

- The repo’s Rust fixture previously printed `WellAligned` twice; it now prints once:

```sh
cargo run --quiet -- inspect \
  tests/fixtures/rust_test/target/debug/rust_test_structs.dSYM/Contents/Resources/DWARF/rust_test_structs \
  --filter WellAligned -o json
```

Observed: exactly 1 matching entry.

- Budget violations are no longer duplicated:

```sh
cat > /tmp/layout_audit_budget_test.yaml <<'EOF'
budgets:
  WellAligned:
    max_size: 1
EOF

cargo run --quiet -- check \
  tests/fixtures/rust_test/target/debug/rust_test_structs.dSYM/Contents/Resources/DWARF/rust_test_structs \
  --config /tmp/layout_audit_budget_test.yaml
```

Observed: `1 violation(s)` (previously `2`).

- The `/tmp` “duplicate Foo” diff becomes stable and semantically correct (one `Foo` changed and the other remained unchanged):

```sh
# v1 binary: /tmp/layout_audit_dup_rust_bin (above)

cat > /tmp/layout_audit_dup_rust_v2.rs <<'RS'
#![allow(dead_code)]

mod a {
    #[repr(C)]
    pub struct Foo {
        pub x: u64,
        pub z: u32,
    }
}

mod b {
    #[repr(C)]
    pub struct Foo {
        pub x: u32,
        pub y: u32,
    }
}

fn main() {
    let a = a::Foo { x: 1, z: 9 };
    let b = b::Foo { x: 2, y: 3 };
    std::hint::black_box(a);
    std::hint::black_box(b);
}
RS
rustc -g /tmp/layout_audit_dup_rust_v2.rs -o /tmp/layout_audit_dup_rust_bin_v2

cargo run --quiet -- \
  diff /tmp/layout_audit_dup_rust_bin.dSYM/Contents/Resources/DWARF/layout_audit_dup_rust_bin \
       /tmp/layout_audit_dup_rust_bin_v2.dSYM/Contents/Resources/DWARF/layout_audit_dup_rust_bin_v2 \
       --filter Foo -o json
```

Observed (summary):
- `changed: 1` (the `Foo` that gained `z`)
- `unchanged_count: 1` (the other `Foo`)
- Stable output across runs.

### Remaining behavior (by design)

`check` budgets are keyed by the displayed struct name. If a binary contains multiple distinct structs with the same `DW_AT_name`, the budget rule for that name will apply to all of them. The audit fixed accidental *duplicate identical entries*, but it does not (and cannot reliably) invent unique names when DWARF doesn’t provide them.

## Finding 3: `suggest --max-align` accepted invalid values and could compute invalid layouts

### Symptom (before)

- `--max-align 0` was accepted, leading to `struct_alignment=0` and invalid results (e.g., “optimized size” values that cannot exist under ABI rules).
- Non-power-of-two `max_align` values (e.g., `3`) could lead to alignments like `3` and offset math that was only correct for power-of-two alignments.

### Root cause (before)

- CLI did not validate `--max-align`.
- `align_up()` used a bitmask-based method that assumes power-of-two alignment.

### Fix

Two-layer hardening:

- CLI rejects `--max-align 0` by adding a `range(1..)` parser (`src/cli.rs`).
- Optimizer clamps `max_align` to at least 1, and `align_up()` now uses a division-based align that works for any positive alignment (`src/analysis/optimize.rs`).

### Validation (after)

- `cargo test` includes new unit tests for non-power-of-two alignment and for clamping behavior.
- `suggest --max-align 0` now fails at argument parsing (instead of producing nonsense output).

## Finding 4: GitHub Action docs mismatch

### Symptom (before)

`action.yml` documented `command` as “inspect, diff, or check”, but the action actually supports `suggest` in its `case` statement.

### Fix

Updated input description to mention `suggest`.

Code: `action.yml`.

## Notes on `unsafe` blocks

The only `unsafe` usage is in DWARF section loading and memory mapping (`src/loader.rs`). During this audit, no concrete unsoundness was identified, and unit/integration tests passed. This remains the most sensitive area of the codebase and should be the first place to look if you see platform-specific crashes on unusual binaries.

---

## Code Review Findings (Dec 15, 2025 - Second Pass)

After the initial audit fixes were committed, a second code review identified additional issues.

## Finding 5: diff.rs code quality issues

### Issues identified

1. **Integer overflow in `member_similarity_score()`**: `i64::try_from()` fallback to `i64::MAX` created incorrect math when subtracting large values.
2. **Magic number for location mismatch penalty**: `i64::MIN / 4` was undocumented.
3. **BTreeMap usage undocumented**: Future maintainers could accidentally switch to HashMap, reintroducing non-determinism.
4. **Wasted O(n*m) similarity calculations**: `scored` vec was built even when only 1 item remained on each side.

### Fix

- Use `u64::abs_diff()` for safe absolute difference calculation
- Use `i128` in `diff_struct()` for safe signed delta
- Add `LOCATION_MISMATCH_PENALTY` constant with documentation
- Add comments explaining BTreeMap is required for determinism
- Move "exactly one remaining" check before building `scored` vec

Code: `src/diff.rs`

## Finding 6: context.rs code quality issues

### Issues identified

1. **Unstable deduplication**: `sort_by` doesn't preserve original order for equal elements; identical fingerprints could keep different items across runs.
2. **Duplicate `read_u64_attr` closures**: Two nearly-identical closures in `process_member()` and `get_source_location()`.
3. **Negative Sdata silently ignored**: Returns `None` without warning for negative values.
4. **No overflow checks in container_offset**: Division could theoretically fail without checked arithmetic.

### Fix

- Add enumerated index as tiebreaker for stable deduplication
- Consolidate closures into `read_u64_from_attr()` helper function with documentation
- Document that negative Sdata values return None (invalid for offsets/sizes)
- Use `checked_div()` and `checked_mul()` for container_offset calculation

Code: `src/dwarf/context.rs`

## Finding 7: optimize.rs edge cases

### Issues identified

1. **Overflow near u64::MAX**: `align_up()` with `saturating_add` could return a value less than input near max range.
2. **Incomplete test coverage**: Missing tests for `alignment=0`, `alignment=1`, larger non-power-of-two, overflow cases.

### Fix

- Use `checked_add()` instead of `saturating_add()`, return `u64::MAX` on overflow
- Added comprehensive test cases covering all edge cases

Code: `src/analysis/optimize.rs`

### Validation

All 81 tests pass. `cargo clippy --all-targets -- -D warnings` is clean.

---

## Code Review Findings (Dec 15, 2025 - Third Pass)

Additional issues found during targeted code review after the second pass fixes.

## Finding 8: optimize.rs critical issues

### Issues identified

1. **Bitfield grouping comparison bug**: `None == None` matched, incorrectly grouping bitfields without known offsets
2. **Bitfield bit_offset invalid after reordering**: Original bit_offset preserved but meaningless in new layout
3. **Offset overflow**: `aligned_offset + unit.total_size` could overflow
4. **Zero-size types silently dropped**: Not added to `skipped_members`
5. **Hardcoded fallback**: Used 4 bytes as default bitfield group size

### Fix

- Only match bitfield offsets when both are `Some` (known values)
- Clear bit_offset after reordering (keep bit_size for reference)
- Use `saturating_add` for offset calculation
- Add zero-size types to `skipped_members` with "(zero-size)" annotation
- Skip groups where size can't be determined reliably

Code: `src/analysis/optimize.rs`

## Finding 9: diff.rs code quality issues

### Issues identified

1. **count_delta overflow**: `unsigned_abs() as i64` could overflow for large differences
2. **HashMap non-determinism**: `diff_struct` used HashMap while rest of module uses BTreeMap
3. **Magic numbers**: Scoring values (5, 2, 1, 10) were undocumented

### Fix

- Use `abs_diff()` with `.min(i64::MAX as usize)` for safe conversion
- Change to BTreeMap for deterministic iteration
- Extract constants: `SCORE_TYPE_MATCH`, `SCORE_SIZE_MATCH`, `SCORE_OFFSET_MATCH`, `SCORE_MEMBER_OVERLAP`

Code: `src/diff.rs`

## Finding 10: context.rs overflow and duplication

### Issues identified

1. **Type resolution duplication**: Nearly identical code in `process_inheritance` and `process_member`
2. **saturating_sub wrong offsets**: Could produce 0 for corrupted DWARF instead of failing
3. **container_offset * 8 overflow**: Unchecked multiplication
4. **raw_bit_offset + bit_size overflow**: Unchecked addition in guard condition

### Fix

- Extract `resolve_type_attr()` helper method
- Use `checked_sub` to detect invalid cross-unit references
- Use `checked_mul` for container_offset * 8
- Use `checked_add` for bitfield boundary check

Code: `src/dwarf/context.rs`

### Validation

All 81 tests pass. `cargo clippy --all-targets -- -D warnings` is clean.

---

## Code Review Findings (Dec 15, 2025 - Fourth Pass)

Additional issues found during continued code review.

## Finding 11: types.rs cross-unit reference handling

### Issue identified

`saturating_sub` was used to compute unit-relative offset from debug_info_offset. If debug_info_offset < unit_debug_offset (corrupted DWARF), this silently returns 0 instead of detecting the error.

### Fix

- Changed `saturating_sub` to `checked_sub`
- Return `Ok(None)` for invalid cross-unit references instead of silently producing offset 0

Code: `src/dwarf/types.rs`

## Finding 12: optimize.rs bitfield tracking bug

### Issues identified

1. **Bitfield members silently lost**: Members in bitfield groups with missing metadata were not being tracked
2. Only successfully converted indices were marked as processed, but unconverted bitfield indices weren't verified

### Fix

- Track `converted_bitfield_indices` separately from `processed_indices`
- After processing all bitfield groups, verify all bitfield indices are accounted for
- Add missing bitfields to `skipped_members` with "(bitfield, missing info)" annotation
- Added test case `test_bitfield_with_missing_metadata_not_lost`

Code: `src/analysis/optimize.rs`

## Finding 13: context.rs attribute parsing duplication

### Issue identified

`process_struct_entry` had duplicated attribute parsing code for `DW_AT_byte_size` and `DW_AT_alignment` that matched multiple `AttributeValue` variants manually, despite `read_u64_from_attr` helper existing.

### Fix

- Replaced manual match patterns with `read_u64_from_attr` calls
- Removed ~10 lines of duplicated code
- Consistent handling of all AttributeValue forms (Sdata, Data1-8, Udata, FileIndex)

Code: `src/dwarf/context.rs`

### Validation

All 82 tests pass. `cargo clippy --all-targets -- -D warnings` is clean.

---

## Code Review Findings (Dec 15, 2025 - Fifth Pass)

Additional issues found during continued code review.

## Finding 14: types.rs array size overflow

### Issue identified

Array size calculation `elem_size * count` could overflow for very large arrays, wrapping to incorrect values.

### Fix

- Use `checked_mul` for array size calculation
- Fall back to `DW_AT_byte_size` if multiplication overflows

Code: `src/dwarf/types.rs`

## Finding 15: types.rs unreachable match arm

### Issue identified

Match arm for type qualifiers (const/volatile/restrict) had `_ => ""` fallback that was unreachable since all three tags were explicitly matched.

### Fix

- Changed to use `_` for the last variant with explicit comment
- Clearer code with no dead branches

Code: `src/dwarf/types.rs`

## Finding 16: context.rs unnecessary clone

### Issue identified

`line_program.clone()` created an unnecessary copy of `IncompleteLineProgram` when only `header()` (a `&self` method) was called.

### Fix

- Changed to `as_ref()` for borrow instead of clone
- Minor performance improvement for source location resolution

Code: `src/dwarf/context.rs`

## Finding 17: Code duplication across DWARF modules

### Issue identified

`read_u64_from_attr` pattern was duplicated in context.rs (explicit function) and types.rs (`get_byte_size`, `extract_count_attr`). The ~20 lines of attribute matching code was repeated multiple times.

### Fix

- Moved `read_u64_from_attr` to `dwarf/mod.rs` as `pub(crate)` function
- Updated context.rs to import from parent module
- Simplified types.rs functions to use shared helper
- Reduced code duplication and ensured consistent attribute handling

Code: `src/dwarf/mod.rs`, `src/dwarf/context.rs`, `src/dwarf/types.rs`

### Validation

All 82 tests pass. `cargo clippy --all-targets -- -D warnings` is clean.

