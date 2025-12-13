use crate::types::{AtomicMember, FalseSharingAnalysis, FalseSharingWarning, StructLayout};
use std::collections::HashMap;

const ATOMIC_PATTERNS: &[&str] = &[
    // Rust std atomics
    "std::sync::atomic::Atomic",
    "core::sync::atomic::Atomic",
    // C++ std::atomic
    "std::atomic<",
    "std::__1::atomic<",
    "std::__cxx11::atomic<",
    // C11 _Atomic
    "_Atomic ",
    // parking_lot
    "parking_lot::Mutex",
    "parking_lot::RwLock",
    "parking_lot::Once",
    "parking_lot::Condvar",
    // crossbeam
    "crossbeam::atomic::AtomicCell",
    "crossbeam_utils::atomic::AtomicCell",
    // atomic_refcell
    "atomic_refcell::AtomicRefCell",
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
            Some(AtomicMember {
                name: m.name.clone(),
                type_name: m.type_name.clone(),
                offset,
                size,
                cache_line: offset / cache_line_size_u64,
            })
        })
        .collect();

    if atomic_members.len() < 2 {
        return FalseSharingAnalysis { atomic_members, warnings: Vec::new() };
    }

    let mut by_cache_line: HashMap<u64, Vec<&AtomicMember>> = HashMap::new();
    for member in &atomic_members {
        by_cache_line.entry(member.cache_line).or_default().push(member);
    }

    let mut warnings = Vec::new();
    for (cache_line, members) in &by_cache_line {
        if members.len() < 2 {
            continue;
        }

        for i in 0..members.len() {
            for j in (i + 1)..members.len() {
                let a = members[i];
                let b = members[j];
                let distance = a.offset.abs_diff(b.offset);

                warnings.push(FalseSharingWarning {
                    member_a: a.name.clone(),
                    member_b: b.name.clone(),
                    cache_line: *cache_line,
                    cache_line_offset: cache_line * cache_line_size_u64,
                    distance,
                });
            }
        }
    }

    warnings.sort_by_key(|w| (w.cache_line, w.member_a.clone(), w.member_b.clone()));

    FalseSharingAnalysis { atomic_members, warnings }
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
}
