use crate::types::{LayoutMetrics, PaddingHole, StructLayout};

/// Analyzes a struct layout for padding holes and cache line metrics.
///
/// # Panics
/// Panics if `cache_line_size` is 0.
pub fn analyze_layout(layout: &mut StructLayout, cache_line_size: u32) {
    assert!(cache_line_size > 0, "cache_line_size must be > 0");
    #[derive(Clone)]
    struct Span {
        start: u64,
        end: u64,
        member_name: String,
    }

    let mut spans = Vec::new();
    let mut partial = false;

    for member in &layout.members {
        let Some(member_offset) = member.offset else {
            partial = true;
            continue;
        };
        let Some(member_size) = member.size else {
            partial = true;
            continue;
        };
        if member_size == 0 {
            continue;
        }

        spans.push(Span {
            start: member_offset,
            end: member_offset.saturating_add(member_size),
            member_name: member.name.clone(),
        });
    }

    spans.sort_by_key(|s| (s.start, s.end));

    let mut padding_holes = Vec::new();
    let mut useful_size: u64 = 0;

    // Can't infer padding without at least one reliable span.
    if spans.is_empty() {
        let cache_line_size_u64 = cache_line_size as u64;
        let cache_lines_spanned =
            if layout.size > 0 { layout.size.div_ceil(cache_line_size_u64) as u32 } else { 0 };

        layout.metrics = LayoutMetrics {
            total_size: layout.size,
            useful_size: 0,
            padding_bytes: 0,
            padding_percentage: 0.0,
            cache_lines_spanned,
            cache_line_density: 0.0,
            padding_holes,
            partial,
            false_sharing: None,
        };
        return;
    }

    // Merge overlapping spans (bitfields share storage, unions overlap, etc.). We use the merged
    // covered bytes for "useful_size" to avoid double-counting overlapping members.
    let mut current_start = spans[0].start;
    let mut current_end = spans[0].end;
    let mut current_end_member: Option<String> = Some(spans[0].member_name.clone());

    for span in spans.into_iter().skip(1) {
        if span.start > current_end {
            useful_size = useful_size.saturating_add(current_end.saturating_sub(current_start));

            if !partial {
                padding_holes.push(PaddingHole {
                    offset: current_end,
                    size: span.start - current_end,
                    after_member: current_end_member.clone(),
                });
            }

            current_start = span.start;
            current_end = span.end;
            current_end_member = Some(span.member_name);
            continue;
        }

        if span.end >= current_end {
            current_end = span.end;
            current_end_member = Some(span.member_name);
        }
    }

    useful_size = useful_size.saturating_add(current_end.saturating_sub(current_start));

    if !partial && current_end < layout.size {
        padding_holes.push(PaddingHole {
            offset: current_end,
            size: layout.size - current_end,
            after_member: current_end_member,
        });
    }

    let padding_bytes: u64 = padding_holes.iter().map(|h| h.size).sum();
    let padding_percentage =
        if layout.size > 0 { (padding_bytes as f64 / layout.size as f64) * 100.0 } else { 0.0 };

    let cache_line_size_u64 = cache_line_size as u64;
    let cache_lines_spanned =
        if layout.size > 0 { layout.size.div_ceil(cache_line_size_u64) as u32 } else { 0 };

    let total_cache_bytes = cache_lines_spanned as u64 * cache_line_size_u64;
    let cache_line_density = if total_cache_bytes > 0 {
        (useful_size as f64 / total_cache_bytes as f64) * 100.0
    } else {
        0.0
    };

    layout.metrics = LayoutMetrics {
        total_size: layout.size,
        useful_size,
        padding_bytes,
        padding_percentage,
        cache_lines_spanned,
        cache_line_density,
        padding_holes,
        partial,
        false_sharing: None,
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::MemberLayout;

    fn make_layout(size: u64, members: Vec<MemberLayout>) -> StructLayout {
        let mut layout = StructLayout::new("TestStruct".to_string(), size, Some(8));
        layout.members = members;
        layout
    }

    #[test]
    fn test_empty_members_no_spans() {
        // Layout with no members at all - exercises the "spans.is_empty()" branch
        let mut layout = make_layout(64, vec![]);

        analyze_layout(&mut layout, 64);

        assert_eq!(layout.metrics.total_size, 64);
        assert_eq!(layout.metrics.useful_size, 0);
        assert_eq!(layout.metrics.padding_bytes, 0);
        assert_eq!(layout.metrics.padding_percentage, 0.0);
        assert_eq!(layout.metrics.cache_lines_spanned, 1);
        assert!(!layout.metrics.partial);
        assert!(layout.metrics.padding_holes.is_empty());
    }

    #[test]
    fn test_zero_size_layout_no_spans() {
        // Layout with size=0 and no members
        let mut layout = make_layout(0, vec![]);

        analyze_layout(&mut layout, 64);

        assert_eq!(layout.metrics.total_size, 0);
        assert_eq!(layout.metrics.cache_lines_spanned, 0);
        assert_eq!(layout.metrics.cache_line_density, 0.0);
    }

    #[test]
    fn test_partial_layout_missing_offset() {
        // Member with missing offset should set partial=true
        let mut layout = make_layout(
            16,
            vec![
                MemberLayout::new("a".to_string(), "u64".to_string(), Some(0), Some(8)),
                MemberLayout::new("b".to_string(), "u64".to_string(), None, Some(8)), // missing offset
            ],
        );

        analyze_layout(&mut layout, 64);

        assert!(layout.metrics.partial);
        // With partial=true, no padding holes should be reported
        assert!(layout.metrics.padding_holes.is_empty());
    }

    #[test]
    fn test_partial_layout_missing_size() {
        // Member with missing size should set partial=true
        let mut layout = make_layout(
            16,
            vec![
                MemberLayout::new("a".to_string(), "u64".to_string(), Some(0), Some(8)),
                MemberLayout::new("b".to_string(), "u64".to_string(), Some(8), None), // missing size
            ],
        );

        analyze_layout(&mut layout, 64);

        assert!(layout.metrics.partial);
        assert!(layout.metrics.padding_holes.is_empty());
    }

    #[test]
    fn test_zero_size_member_skipped() {
        // Zero-size members should be skipped (not contribute to spans)
        let mut layout = make_layout(
            16,
            vec![
                MemberLayout::new("a".to_string(), "u64".to_string(), Some(0), Some(8)),
                MemberLayout::new("zst".to_string(), "()".to_string(), Some(8), Some(0)), // ZST
                MemberLayout::new("b".to_string(), "u64".to_string(), Some(8), Some(8)),
            ],
        );

        analyze_layout(&mut layout, 64);

        assert!(!layout.metrics.partial);
        assert_eq!(layout.metrics.useful_size, 16);
    }

    #[test]
    fn test_overlapping_spans_merged() {
        // Overlapping members (e.g., union-like) should be merged, not double-counted
        let mut layout = make_layout(
            16,
            vec![
                MemberLayout::new("a".to_string(), "u64".to_string(), Some(0), Some(16)),
                MemberLayout::new("b".to_string(), "u64".to_string(), Some(4), Some(8)), // overlaps with a
            ],
        );

        analyze_layout(&mut layout, 64);

        // Should count as 16 bytes useful, not 24
        assert_eq!(layout.metrics.useful_size, 16);
        assert!(layout.metrics.padding_holes.is_empty());
    }

    #[test]
    fn test_padding_hole_detected() {
        // Gap between members should create a padding hole
        let mut layout = make_layout(
            24,
            vec![
                MemberLayout::new("a".to_string(), "u64".to_string(), Some(0), Some(8)),
                // 8 bytes gap
                MemberLayout::new("b".to_string(), "u64".to_string(), Some(16), Some(8)),
            ],
        );

        analyze_layout(&mut layout, 64);

        assert_eq!(layout.metrics.padding_bytes, 8);
        assert_eq!(layout.metrics.padding_holes.len(), 1);
        assert_eq!(layout.metrics.padding_holes[0].offset, 8);
        assert_eq!(layout.metrics.padding_holes[0].size, 8);
    }

    #[test]
    fn test_tail_padding_detected() {
        // Gap at end of struct is tail padding
        let mut layout = make_layout(
            16,
            vec![MemberLayout::new("a".to_string(), "u64".to_string(), Some(0), Some(8))],
        );

        analyze_layout(&mut layout, 64);

        assert_eq!(layout.metrics.padding_bytes, 8);
        assert_eq!(layout.metrics.padding_holes.len(), 1);
        assert_eq!(layout.metrics.padding_holes[0].offset, 8);
    }

    #[test]
    fn test_partial_layout_no_tail_padding_reported() {
        // With partial=true, tail padding should NOT be reported
        let mut layout = make_layout(
            32,
            vec![
                MemberLayout::new("a".to_string(), "u64".to_string(), Some(0), Some(8)),
                MemberLayout::new("b".to_string(), "u64".to_string(), None, Some(8)), // missing offset
            ],
        );

        analyze_layout(&mut layout, 64);

        assert!(layout.metrics.partial);
        // No padding holes when partial (can't reliably detect them)
        assert!(layout.metrics.padding_holes.is_empty());
    }
}
