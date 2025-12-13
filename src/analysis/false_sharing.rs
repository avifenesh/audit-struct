use crate::types::{
    AtomicMember, CacheLineSpanningWarning, FalseSharingAnalysis, FalseSharingWarning, StructLayout,
};
use std::collections::HashMap;

const ATOMIC_PATTERNS: &[&str] = &[
    // Rust std atomics (full paths)
    "std::sync::atomic::Atomic",
    "core::sync::atomic::Atomic",
    // Rust std sync primitives (these use internal atomics)
    "std::sync::Mutex",
    "std::sync::RwLock",
    "std::sync::Condvar",
    "std::sync::Once",
    "std::sync::OnceLock",
    "std::sync::Barrier",
    // C++ std::atomic (various implementations)
    "std::atomic<",
    "std::__1::atomic<",
    "std::__cxx11::atomic<",
    // C11 _Atomic (with space or parenthesized)
    "_Atomic ",
    "_Atomic(",
    // parking_lot
    "parking_lot::Mutex",
    "parking_lot::RwLock",
    "parking_lot::Once",
    "parking_lot::Condvar",
    "parking_lot::ReentrantMutex",
    "parking_lot::FairMutex",
    "parking_lot::RawMutex",
    "parking_lot::RawRwLock",
    // crossbeam
    "crossbeam::atomic::AtomicCell",
    "crossbeam_utils::atomic::AtomicCell",
    "crossbeam_epoch::Atomic",
    // atomic_refcell
    "atomic_refcell::AtomicRefCell",
    // tokio sync primitives
    "tokio::sync::Mutex",
    "tokio::sync::RwLock",
    "tokio::sync::Semaphore",
    "tokio::sync::Notify",
    "tokio::sync::Barrier",
    "tokio::sync::OnceCell",
    // arc_swap
    "arc_swap::ArcSwap",
    "arc_swap::ArcSwapOption",
    "arc_swap::ArcSwapAny",
];

fn is_atomic_type(type_name: &str) -> bool {
    ATOMIC_PATTERNS.iter().any(|pattern| type_name.contains(pattern))
}

pub fn analyze_false_sharing(layout: &StructLayout, cache_line_size: u32) -> FalseSharingAnalysis {
    let cache_line_size_u64 = cache_line_size as u64;

    let atomic_members: Vec<AtomicMember> = layout
        .members
        .iter()
        .filter(|m| is_atomic_type(&m.type_name))
        .filter_map(|m| {
            let offset = m.offset?;
            let size = m.size?;
            if size == 0 {
                return None;
            }
            let cache_line = offset / cache_line_size_u64;
            let end_offset = offset + size - 1; // Last byte of the member
            let end_cache_line = end_offset / cache_line_size_u64;
            let spans_cache_lines = end_cache_line > cache_line;

            Some(AtomicMember {
                name: m.name.clone(),
                type_name: m.type_name.clone(),
                offset,
                size,
                cache_line,
                end_cache_line,
                spans_cache_lines,
            })
        })
        .collect();

    if atomic_members.is_empty() {
        return FalseSharingAnalysis::default();
    }

    // Generate spanning warnings for atomics that cross cache line boundaries
    let spanning_warnings: Vec<CacheLineSpanningWarning> = atomic_members
        .iter()
        .filter(|m| m.spans_cache_lines)
        .map(|m| CacheLineSpanningWarning {
            member: m.name.clone(),
            type_name: m.type_name.clone(),
            offset: m.offset,
            size: m.size,
            start_cache_line: m.cache_line,
            end_cache_line: m.end_cache_line,
            lines_spanned: m.end_cache_line - m.cache_line + 1,
        })
        .collect();

    if atomic_members.len() < 2 {
        return FalseSharingAnalysis { atomic_members, warnings: Vec::new(), spanning_warnings };
    }

    // Group atomics by all cache lines they touch (not just start)
    let mut by_cache_line: HashMap<u64, Vec<&AtomicMember>> = HashMap::new();
    for member in &atomic_members {
        for cache_line in member.cache_line..=member.end_cache_line {
            by_cache_line.entry(cache_line).or_default().push(member);
        }
    }

    let mut warnings = Vec::new();
    let mut seen_pairs: std::collections::HashSet<(&str, &str)> = std::collections::HashSet::new();

    for (cache_line, members) in &by_cache_line {
        if members.len() < 2 {
            continue;
        }

        for i in 0..members.len() {
            for j in (i + 1)..members.len() {
                let a = members[i];
                let b = members[j];

                // Ensure consistent ordering and deduplicate
                let (first, second) = if a.offset <= b.offset { (a, b) } else { (b, a) };

                let pair_key = (first.name.as_str(), second.name.as_str());
                if seen_pairs.contains(&pair_key) {
                    continue;
                }
                seen_pairs.insert(pair_key);

                // gap_bytes = second.offset - (first.offset + first.size)
                // Negative = overlap, Zero = adjacent, Positive = gap
                let first_end = first.offset + first.size;
                let gap_bytes = second.offset as i64 - first_end as i64;

                warnings.push(FalseSharingWarning {
                    member_a: first.name.clone(),
                    member_b: second.name.clone(),
                    cache_line: *cache_line,
                    gap_bytes,
                });
            }
        }
    }

    // Sort by (cache_line, member_a, member_b) without cloning strings
    warnings.sort_by(|a, b| {
        a.cache_line
            .cmp(&b.cache_line)
            .then_with(|| a.member_a.cmp(&b.member_a))
            .then_with(|| a.member_b.cmp(&b.member_b))
    });

    FalseSharingAnalysis { atomic_members, warnings, spanning_warnings }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::MemberLayout;

    fn make_layout_with_members(members: Vec<MemberLayout>) -> StructLayout {
        let mut layout = StructLayout::new("TestStruct".to_string(), 128, Some(8));
        layout.members = members;
        layout
    }

    #[test]
    fn test_two_atomics_same_cache_line() {
        let layout = make_layout_with_members(vec![
            MemberLayout::new(
                "counter".to_string(),
                "std::sync::atomic::AtomicU64".to_string(),
                Some(0),
                Some(8),
            ),
            MemberLayout::new(
                "flag".to_string(),
                "std::sync::atomic::AtomicBool".to_string(),
                Some(8),
                Some(1),
            ),
        ]);

        let analysis = analyze_false_sharing(&layout, 64);

        assert_eq!(analysis.atomic_members.len(), 2);
        assert_eq!(analysis.warnings.len(), 1);
        assert_eq!(analysis.warnings[0].cache_line, 0);
        assert_eq!(analysis.warnings[0].member_a, "counter");
        assert_eq!(analysis.warnings[0].member_b, "flag");
        assert_eq!(analysis.warnings[0].gap_bytes, 0); // Adjacent
    }

    #[test]
    fn test_two_atomics_different_cache_lines() {
        let layout = make_layout_with_members(vec![
            MemberLayout::new(
                "counter1".to_string(),
                "std::sync::atomic::AtomicU64".to_string(),
                Some(0),
                Some(8),
            ),
            MemberLayout::new(
                "counter2".to_string(),
                "std::sync::atomic::AtomicU64".to_string(),
                Some(64),
                Some(8),
            ),
        ]);

        let analysis = analyze_false_sharing(&layout, 64);

        assert_eq!(analysis.atomic_members.len(), 2);
        assert!(analysis.warnings.is_empty());
    }

    #[test]
    fn test_three_atomics_same_cache_line() {
        let layout = make_layout_with_members(vec![
            MemberLayout::new(
                "a".to_string(),
                "std::sync::atomic::AtomicU64".to_string(),
                Some(0),
                Some(8),
            ),
            MemberLayout::new(
                "b".to_string(),
                "std::sync::atomic::AtomicU64".to_string(),
                Some(8),
                Some(8),
            ),
            MemberLayout::new(
                "c".to_string(),
                "std::sync::atomic::AtomicU64".to_string(),
                Some(16),
                Some(8),
            ),
        ]);

        let analysis = analyze_false_sharing(&layout, 64);

        assert_eq!(analysis.atomic_members.len(), 3);
        assert_eq!(analysis.warnings.len(), 3); // (a,b), (a,c), (b,c)
    }

    #[test]
    fn test_non_atomic_ignored() {
        let layout = make_layout_with_members(vec![
            MemberLayout::new(
                "counter".to_string(),
                "std::sync::atomic::AtomicU64".to_string(),
                Some(0),
                Some(8),
            ),
            MemberLayout::new("data".to_string(), "u64".to_string(), Some(8), Some(8)),
        ]);

        let analysis = analyze_false_sharing(&layout, 64);

        assert_eq!(analysis.atomic_members.len(), 1);
        assert!(analysis.warnings.is_empty());
    }

    #[test]
    fn test_cpp_atomic_detection() {
        let layout = make_layout_with_members(vec![
            MemberLayout::new("a".to_string(), "std::atomic<int>".to_string(), Some(0), Some(4)),
            MemberLayout::new("b".to_string(), "std::atomic<int>".to_string(), Some(4), Some(4)),
        ]);

        let analysis = analyze_false_sharing(&layout, 64);

        assert_eq!(analysis.atomic_members.len(), 2);
        assert_eq!(analysis.warnings.len(), 1);
    }

    #[test]
    fn test_c11_atomic_detection() {
        let layout = make_layout_with_members(vec![
            MemberLayout::new("a".to_string(), "_Atomic int".to_string(), Some(0), Some(4)),
            MemberLayout::new("b".to_string(), "_Atomic int".to_string(), Some(4), Some(4)),
        ]);

        let analysis = analyze_false_sharing(&layout, 64);

        assert_eq!(analysis.atomic_members.len(), 2);
        assert_eq!(analysis.warnings.len(), 1);
    }

    #[test]
    fn test_parking_lot_detection() {
        let layout = make_layout_with_members(vec![
            MemberLayout::new(
                "lock1".to_string(),
                "parking_lot::Mutex<T>".to_string(),
                Some(0),
                Some(8),
            ),
            MemberLayout::new(
                "lock2".to_string(),
                "parking_lot::RwLock<T>".to_string(),
                Some(8),
                Some(16),
            ),
        ]);

        let analysis = analyze_false_sharing(&layout, 64);

        assert_eq!(analysis.atomic_members.len(), 2);
        assert_eq!(analysis.warnings.len(), 1);
    }

    // New tests for P2

    #[test]
    fn test_std_sync_mutex_detection() {
        let layout = make_layout_with_members(vec![
            MemberLayout::new(
                "lock1".to_string(),
                "std::sync::Mutex<i32>".to_string(),
                Some(0),
                Some(16),
            ),
            MemberLayout::new(
                "lock2".to_string(),
                "std::sync::RwLock<i32>".to_string(),
                Some(16),
                Some(24),
            ),
        ]);

        let analysis = analyze_false_sharing(&layout, 64);

        assert_eq!(analysis.atomic_members.len(), 2);
        assert_eq!(analysis.warnings.len(), 1);
    }

    #[test]
    fn test_single_atomic_no_warnings() {
        let layout = make_layout_with_members(vec![MemberLayout::new(
            "counter".to_string(),
            "std::sync::atomic::AtomicU64".to_string(),
            Some(0),
            Some(8),
        )]);

        let analysis = analyze_false_sharing(&layout, 64);

        assert_eq!(analysis.atomic_members.len(), 1);
        assert!(analysis.warnings.is_empty());
        assert!(analysis.spanning_warnings.is_empty());
    }

    #[test]
    fn test_zero_size_atomic_ignored() {
        let layout = make_layout_with_members(vec![
            MemberLayout::new(
                "counter".to_string(),
                "std::sync::atomic::AtomicU64".to_string(),
                Some(0),
                Some(8),
            ),
            MemberLayout::new(
                "zst".to_string(),
                "std::sync::atomic::AtomicUnit".to_string(), // hypothetical ZST
                Some(8),
                Some(0),
            ),
        ]);

        let analysis = analyze_false_sharing(&layout, 64);

        assert_eq!(analysis.atomic_members.len(), 1);
        assert!(analysis.warnings.is_empty());
    }

    #[test]
    fn test_c11_atomic_parenthesized() {
        let layout = make_layout_with_members(vec![
            MemberLayout::new("a".to_string(), "_Atomic(int)".to_string(), Some(0), Some(4)),
            MemberLayout::new("b".to_string(), "_Atomic(int)".to_string(), Some(4), Some(4)),
        ]);

        let analysis = analyze_false_sharing(&layout, 64);

        assert_eq!(analysis.atomic_members.len(), 2);
        assert_eq!(analysis.warnings.len(), 1);
    }

    #[test]
    fn test_tokio_sync_detection() {
        let layout = make_layout_with_members(vec![
            MemberLayout::new(
                "lock1".to_string(),
                "tokio::sync::Mutex<i32>".to_string(),
                Some(0),
                Some(16),
            ),
            MemberLayout::new(
                "lock2".to_string(),
                "tokio::sync::RwLock<i32>".to_string(),
                Some(16),
                Some(24),
            ),
        ]);

        let analysis = analyze_false_sharing(&layout, 64);

        assert_eq!(analysis.atomic_members.len(), 2);
        assert_eq!(analysis.warnings.len(), 1);
    }

    // Tests for P3: cache line spanning

    #[test]
    fn test_atomic_spanning_cache_lines() {
        // An atomic at offset 60 with size 8 spans bytes 60-67, crossing the 64-byte boundary
        let layout = make_layout_with_members(vec![MemberLayout::new(
            "spanning".to_string(),
            "std::sync::atomic::AtomicU64".to_string(),
            Some(60),
            Some(8),
        )]);

        let analysis = analyze_false_sharing(&layout, 64);

        assert_eq!(analysis.atomic_members.len(), 1);
        assert!(analysis.atomic_members[0].spans_cache_lines);
        assert_eq!(analysis.atomic_members[0].cache_line, 0);
        assert_eq!(analysis.atomic_members[0].end_cache_line, 1);

        assert_eq!(analysis.spanning_warnings.len(), 1);
        assert_eq!(analysis.spanning_warnings[0].member, "spanning");
        assert_eq!(analysis.spanning_warnings[0].lines_spanned, 2);
    }

    #[test]
    fn test_atomic_not_spanning() {
        // An atomic at offset 0 with size 8 stays within cache line 0
        let layout = make_layout_with_members(vec![MemberLayout::new(
            "aligned".to_string(),
            "std::sync::atomic::AtomicU64".to_string(),
            Some(0),
            Some(8),
        )]);

        let analysis = analyze_false_sharing(&layout, 64);

        assert_eq!(analysis.atomic_members.len(), 1);
        assert!(!analysis.atomic_members[0].spans_cache_lines);
        assert!(analysis.spanning_warnings.is_empty());
    }

    #[test]
    fn test_gap_bytes_calculation() {
        let layout = make_layout_with_members(vec![
            MemberLayout::new(
                "a".to_string(),
                "std::sync::atomic::AtomicU64".to_string(),
                Some(0),
                Some(8),
            ),
            MemberLayout::new(
                "b".to_string(),
                "std::sync::atomic::AtomicU64".to_string(),
                Some(16), // 8-byte gap between a (ends at 8) and b (starts at 16)
                Some(8),
            ),
        ]);

        let analysis = analyze_false_sharing(&layout, 64);

        assert_eq!(analysis.warnings.len(), 1);
        assert_eq!(analysis.warnings[0].gap_bytes, 8); // Positive gap
    }

    #[test]
    fn test_gap_bytes_adjacent() {
        let layout = make_layout_with_members(vec![
            MemberLayout::new(
                "a".to_string(),
                "std::sync::atomic::AtomicU64".to_string(),
                Some(0),
                Some(8),
            ),
            MemberLayout::new(
                "b".to_string(),
                "std::sync::atomic::AtomicU64".to_string(),
                Some(8), // Adjacent: a ends at 8, b starts at 8
                Some(8),
            ),
        ]);

        let analysis = analyze_false_sharing(&layout, 64);

        assert_eq!(analysis.warnings.len(), 1);
        assert_eq!(analysis.warnings[0].gap_bytes, 0); // Zero = adjacent
    }

    #[test]
    fn test_spanning_atomic_shares_with_both_lines() {
        // Atomic at offset 60-67 spans cache lines 0 and 1
        // Another atomic at offset 70 is on cache line 1
        // They should produce a warning for cache line 1
        let layout = make_layout_with_members(vec![
            MemberLayout::new(
                "spanning".to_string(),
                "std::sync::atomic::AtomicU64".to_string(),
                Some(60),
                Some(8),
            ),
            MemberLayout::new(
                "other".to_string(),
                "std::sync::atomic::AtomicU64".to_string(),
                Some(70),
                Some(8),
            ),
        ]);

        let analysis = analyze_false_sharing(&layout, 64);

        assert_eq!(analysis.atomic_members.len(), 2);
        // One warning for the pair on cache line 1
        assert_eq!(analysis.warnings.len(), 1);
        assert_eq!(analysis.warnings[0].cache_line, 1);
    }
}
