//! Field reordering optimization for struct layouts.

use crate::types::{MemberLayout, StructLayout};
use serde::Serialize;
use std::collections::HashSet;

/// Result of optimizing a struct layout.
#[derive(Debug, Clone, Serialize)]
pub struct OptimizedLayout {
    pub name: String,
    pub original_size: u64,
    pub optimized_size: u64,
    pub savings_bytes: u64,
    pub savings_percent: f64,
    pub struct_alignment: u64,
    pub original_members: Vec<OptimizedMember>,
    pub optimized_members: Vec<OptimizedMember>,
    /// Members that could not be optimized (missing size/offset).
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub skipped_members: Vec<String>,
    /// True if layout contains bitfields that were kept together.
    pub has_bitfields: bool,
}

/// Member with computed offset and alignment.
#[derive(Debug, Clone, Serialize)]
pub struct OptimizedMember {
    pub name: String,
    pub type_name: String,
    pub offset: u64,
    pub size: u64,
    pub alignment: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bit_offset: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bit_size: Option<u64>,
}

/// Infer alignment from size using standard C ABI rules.
/// Returns alignment as power of 2, capped at max_align.
pub fn infer_alignment(size: u64, max_align: u64) -> u64 {
    if size == 0 {
        return 1;
    }
    // For primitives: alignment = min(size rounded to power of 2, max_align)
    let natural_align = size.next_power_of_two();
    natural_align.min(max_align).max(1)
}

/// Align value up to alignment boundary.
/// Returns value unchanged if alignment <= 1.
/// For values near u64::MAX where alignment would overflow, returns u64::MAX.
fn align_up(value: u64, alignment: u64) -> u64 {
    if alignment <= 1 {
        return value;
    }
    // Works for any positive alignment (not only powers of two).
    // Use checked arithmetic to detect overflow near u64::MAX.
    match value.checked_add(alignment - 1) {
        Some(sum) => (sum / alignment) * alignment,
        // Overflow: value is near u64::MAX and can't be aligned up safely.
        // Return u64::MAX as a safe upper bound (real structs won't hit this).
        None => u64::MAX,
    }
}

/// A sortable unit for optimization - either a single member or a bitfield group.
#[derive(Clone)]
struct SortableUnit {
    members: Vec<OptimizedMember>,
    total_size: u64,
    alignment: u64,
}

/// Group bitfield members that share storage units.
/// Returns indices of members belonging to each bitfield group.
/// Note: Members in bitfield groups may still be filtered out during optimization
/// if they have missing size/offset. Callers must track converted indices separately.
fn find_bitfield_groups(members: &[MemberLayout]) -> Vec<Vec<usize>> {
    let mut groups: Vec<Vec<usize>> = Vec::new();
    let mut current_group: Vec<usize> = Vec::new();
    let mut current_offset: Option<u64> = None;

    for (idx, member) in members.iter().enumerate() {
        if member.bit_size.is_some() {
            let base_offset = member.offset;
            // Only group bitfields with known, matching offsets.
            // None == None should NOT match (can't verify they share storage).
            let same_storage = match (current_offset, base_offset) {
                (Some(curr), Some(base)) => curr == base,
                _ => false,
            };
            if same_storage && !current_group.is_empty() {
                // Same storage unit
                current_group.push(idx);
            } else {
                // New storage unit
                if !current_group.is_empty() {
                    groups.push(std::mem::take(&mut current_group));
                }
                current_group.push(idx);
                current_offset = base_offset;
            }
        } else if !current_group.is_empty() {
            // Non-bitfield breaks the group
            groups.push(std::mem::take(&mut current_group));
            current_offset = None;
        }
    }

    if !current_group.is_empty() {
        groups.push(current_group);
    }

    groups
}

/// Optimize a struct layout by reordering fields to minimize padding.
/// Uses greedy bin-packing: sort by alignment desc, then size desc.
pub fn optimize_layout(layout: &StructLayout, max_align: u64) -> OptimizedLayout {
    let max_align = max_align.max(1);
    // If struct alignment is known, use it; otherwise infer from member alignments
    let inferred_alignment = layout
        .members
        .iter()
        .filter_map(|m| m.size)
        .map(|s| infer_alignment(s, max_align))
        .max()
        .unwrap_or(1);

    let struct_alignment = layout.alignment.unwrap_or(inferred_alignment).min(max_align);

    // Find bitfield groups
    let bitfield_groups = find_bitfield_groups(&layout.members);
    let bitfield_indices: HashSet<usize> = bitfield_groups.iter().flatten().copied().collect();
    let has_bitfields = !bitfield_groups.is_empty();

    // Build original members list with inferred alignment
    let mut original_members: Vec<OptimizedMember> = Vec::new();
    let mut skipped_members: Vec<String> = Vec::new();

    for member in &layout.members {
        let Some(size) = member.size else {
            skipped_members.push(member.name.clone());
            continue;
        };
        let Some(offset) = member.offset else {
            skipped_members.push(member.name.clone());
            continue;
        };
        // Skip zero-size types (ZSTs don't affect layout)
        if size == 0 {
            skipped_members.push(format!("{} (zero-size)", member.name));
            continue;
        }

        let alignment = infer_alignment(size, max_align);

        original_members.push(OptimizedMember {
            name: member.name.clone(),
            type_name: member.type_name.clone(),
            offset,
            size,
            alignment,
            bit_offset: member.bit_offset,
            bit_size: member.bit_size,
        });
    }

    // Build sortable units
    let mut units: Vec<SortableUnit> = Vec::new();
    let mut processed_indices: HashSet<usize> = HashSet::new();

    // Track which bitfield indices were successfully converted to OptimizedMember.
    // This allows us to verify no bitfield member is silently lost.
    let mut converted_bitfield_indices: HashSet<usize> = HashSet::new();

    // Add bitfield groups as units
    for group in &bitfield_groups {
        if group.is_empty() {
            continue;
        }

        // Track which indices in this group were successfully converted
        let mut group_converted: Vec<usize> = Vec::new();
        let group_members: Vec<OptimizedMember> = group
            .iter()
            .filter_map(|&idx| {
                layout.members.get(idx).and_then(|lm| {
                    original_members.iter().find(|m| m.name == lm.name).cloned().inspect(|_| {
                        group_converted.push(idx);
                    })
                })
            })
            .collect();

        if group_members.is_empty() {
            continue;
        }

        // Bitfield group size = storage unit size (from first member)
        // Skip groups where we can't determine size reliably.
        let Some(total_size) = group_members.first().map(|m| m.size) else {
            continue;
        };
        let alignment = group_members.iter().map(|m| m.alignment).max().unwrap_or(1);

        // Mark only successfully converted indices as processed
        for idx in &group_converted {
            processed_indices.insert(*idx);
            converted_bitfield_indices.insert(*idx);
        }

        units.push(SortableUnit { members: group_members, total_size, alignment });
    }

    // Verify all bitfield indices are accounted for (either converted or in skipped_members).
    // This defensive check prevents silent data loss from edge cases.
    // Collect names to add first to avoid borrow conflicts.
    let missing_bitfield_names: Vec<String> = {
        let skipped_names: HashSet<&str> = skipped_members.iter().map(|s| s.as_str()).collect();
        bitfield_indices
            .iter()
            .filter(|&&idx| !converted_bitfield_indices.contains(&idx))
            .filter_map(|&idx| layout.members.get(idx))
            .filter(|member| {
                // Check if already in skipped_members (may have "(zero-size)" suffix)
                !skipped_names.contains(member.name.as_str())
                    && !skipped_members.iter().any(|s| s.starts_with(&member.name))
            })
            .map(|member| format!("{} (bitfield, missing info)", member.name))
            .collect()
    };
    skipped_members.extend(missing_bitfield_names);

    // Add non-bitfield members as single-member units
    for (idx, member) in layout.members.iter().enumerate() {
        if processed_indices.contains(&idx) || bitfield_indices.contains(&idx) {
            continue;
        }

        if let Some(opt_member) = original_members.iter().find(|m| m.name == member.name) {
            units.push(SortableUnit {
                members: vec![opt_member.clone()],
                total_size: opt_member.size,
                alignment: opt_member.alignment,
            });
        }
    }

    // Sort: largest alignment first, then largest size
    units.sort_by(|a, b| {
        b.alignment.cmp(&a.alignment).then_with(|| b.total_size.cmp(&a.total_size))
    });

    // Place members greedily
    let mut optimized_members: Vec<OptimizedMember> = Vec::new();
    let mut current_offset: u64 = 0;

    for unit in units {
        // Align to unit's alignment requirement
        let aligned_offset = align_up(current_offset, unit.alignment);

        for mut member in unit.members {
            member.offset = aligned_offset;
            // Clear bit_offset after reordering - the original value was relative to
            // the original layout and is no longer valid. Keep bit_size for reference.
            if member.bit_size.is_some() {
                member.bit_offset = None;
            }
            optimized_members.push(member);
        }

        // Use saturating_add to prevent overflow near u64::MAX
        current_offset = aligned_offset.saturating_add(unit.total_size);
    }

    // Add tail padding to reach struct alignment
    let optimized_size = align_up(current_offset, struct_alignment);

    let savings_bytes = layout.size.saturating_sub(optimized_size);
    let savings_percent =
        if layout.size > 0 { (savings_bytes as f64 / layout.size as f64) * 100.0 } else { 0.0 };

    OptimizedLayout {
        name: layout.name.clone(),
        original_size: layout.size,
        optimized_size,
        savings_bytes,
        savings_percent,
        struct_alignment,
        original_members,
        optimized_members,
        skipped_members,
        has_bitfields,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_infer_alignment() {
        assert_eq!(infer_alignment(1, 8), 1); // char
        assert_eq!(infer_alignment(2, 8), 2); // short
        assert_eq!(infer_alignment(4, 8), 4); // int
        assert_eq!(infer_alignment(8, 8), 8); // long/pointer
        assert_eq!(infer_alignment(16, 8), 8); // capped at max_align
        assert_eq!(infer_alignment(3, 8), 4); // rounds up to power of 2
        assert_eq!(infer_alignment(0, 8), 1); // ZST
    }

    #[test]
    fn test_align_up() {
        // Basic cases
        assert_eq!(align_up(0, 4), 0);
        assert_eq!(align_up(1, 4), 4);
        assert_eq!(align_up(4, 4), 4);
        assert_eq!(align_up(5, 4), 8);
        assert_eq!(align_up(7, 8), 8);

        // Non-power-of-two alignments
        assert_eq!(align_up(4, 3), 6);
        assert_eq!(align_up(6, 3), 6);
        assert_eq!(align_up(10, 12), 12);
        assert_eq!(align_up(24, 12), 24);

        // Edge case: alignment = 0 (returns value unchanged)
        assert_eq!(align_up(10, 0), 10);

        // Edge case: alignment = 1 (returns value unchanged)
        assert_eq!(align_up(5, 1), 5);
        assert_eq!(align_up(0, 1), 0);

        // Overflow protection: value near u64::MAX
        assert_eq!(align_up(u64::MAX - 5, 16), u64::MAX);
        assert_eq!(align_up(u64::MAX, 8), u64::MAX);
    }

    #[test]
    fn test_optimize_padded_struct() {
        // struct { char a; int b; char c; } = 12 bytes with padding
        // optimal: struct { int b; char a; char c; } = 8 bytes
        let mut layout = StructLayout::new("Test".to_string(), 12, Some(4));
        layout.members = vec![
            MemberLayout::new("a".to_string(), "char".to_string(), Some(0), Some(1)),
            MemberLayout::new("b".to_string(), "int".to_string(), Some(4), Some(4)),
            MemberLayout::new("c".to_string(), "char".to_string(), Some(8), Some(1)),
        ];

        let result = optimize_layout(&layout, 8);

        assert_eq!(result.original_size, 12);
        assert_eq!(result.optimized_size, 8);
        assert_eq!(result.savings_bytes, 4);

        // First member should be the int (largest alignment)
        assert_eq!(result.optimized_members[0].name, "b");
        assert_eq!(result.optimized_members[0].offset, 0);
    }

    #[test]
    fn test_already_optimal() {
        // struct { int a; int b; } = 8 bytes, already optimal
        let mut layout = StructLayout::new("Test".to_string(), 8, Some(4));
        layout.members = vec![
            MemberLayout::new("a".to_string(), "int".to_string(), Some(0), Some(4)),
            MemberLayout::new("b".to_string(), "int".to_string(), Some(4), Some(4)),
        ];

        let result = optimize_layout(&layout, 8);

        assert_eq!(result.original_size, 8);
        assert_eq!(result.optimized_size, 8);
        assert_eq!(result.savings_bytes, 0);
    }

    #[test]
    fn test_skipped_members() {
        let mut layout = StructLayout::new("Test".to_string(), 16, Some(8));
        layout.members = vec![
            MemberLayout::new("a".to_string(), "int".to_string(), Some(0), Some(4)),
            MemberLayout::new("b".to_string(), "unknown".to_string(), None, None), // missing info
        ];

        let result = optimize_layout(&layout, 8);

        assert_eq!(result.skipped_members, vec!["b"]);
    }

    #[test]
    fn test_max_align_zero_is_safely_clamped() {
        let mut layout = StructLayout::new("Test".to_string(), 12, Some(4));
        layout.members = vec![
            MemberLayout::new("a".to_string(), "char".to_string(), Some(0), Some(1)),
            MemberLayout::new("b".to_string(), "int".to_string(), Some(4), Some(4)),
        ];

        let result = optimize_layout(&layout, 0);
        assert!(result.struct_alignment >= 1);
        assert!(result.optimized_size > 0);
    }

    #[test]
    fn test_bitfield_with_missing_metadata_not_lost() {
        // Test that bitfield members with missing metadata are tracked in skipped_members
        let mut layout = StructLayout::new("Test".to_string(), 8, Some(4));

        // Create a bitfield member with valid info
        let mut valid_bitfield =
            MemberLayout::new("flags".to_string(), "unsigned int".to_string(), Some(0), Some(4));
        valid_bitfield.bit_size = Some(3);
        valid_bitfield.bit_offset = Some(0);

        // Create a bitfield member with missing metadata (same offset = same storage unit)
        let mut missing_bitfield =
            MemberLayout::new("status".to_string(), "unsigned int".to_string(), Some(0), None);
        missing_bitfield.bit_size = Some(2);

        layout.members = vec![valid_bitfield, missing_bitfield];

        let result = optimize_layout(&layout, 8);

        // The valid bitfield should be optimized
        assert!(result.optimized_members.iter().any(|m| m.name == "flags"));

        // The missing bitfield should be in skipped_members (not silently lost)
        assert!(
            result.skipped_members.iter().any(|s| s.contains("status")),
            "Bitfield 'status' with missing metadata should be in skipped_members, got: {:?}",
            result.skipped_members
        );
    }
}
