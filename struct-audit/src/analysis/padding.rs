use crate::types::{LayoutMetrics, PaddingHole, StructLayout};

pub fn analyze_layout(layout: &mut StructLayout, cache_line_size: u32) {
    let mut padding_holes = Vec::new();
    let mut useful_size: u64 = 0;
    let mut current_offset: u64 = 0;
    let mut last_member_name: Option<String> = None;
    let mut partial = false;

    for member in &layout.members {
        let Some(member_offset) = member.offset else {
            partial = true;
            last_member_name = Some(member.name.clone());
            continue;
        };

        let Some(member_size) = member.size else {
            partial = true;
            last_member_name = Some(member.name.clone());
            continue;
        };

        if member_offset > current_offset {
            let gap_size = member_offset - current_offset;
            padding_holes.push(PaddingHole {
                offset: current_offset,
                size: gap_size,
                after_member: last_member_name.clone(),
            });
        }

        useful_size += member_size;
        current_offset = current_offset.max(member_offset + member_size);
        last_member_name = Some(member.name.clone());
    }

    // Only report tail padding if we have reliable offset information.
    // When partial=true and current_offset=0, all members had unknown offsets
    // (e.g., bitfields), so we can't reliably compute padding.
    let has_known_members = current_offset > 0 || !partial;
    if has_known_members && current_offset < layout.size {
        let tail_padding = layout.size - current_offset;
        padding_holes.push(PaddingHole {
            offset: current_offset,
            size: tail_padding,
            after_member: last_member_name,
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
    };
}
