use crate::types::{LayoutMetrics, PaddingHole, StructLayout};

pub fn analyze_layout(layout: &mut StructLayout, cache_line_size: u32) {
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
