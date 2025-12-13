use crate::types::StructLayout;
use colored::Colorize;
use comfy_table::{Cell, CellAlignment, Color, Table, presets::UTF8_FULL_CONDENSED};

pub struct TableFormatter {
    no_color: bool,
    cache_line_size: u32,
}

impl TableFormatter {
    pub fn new(no_color: bool, cache_line_size: u32) -> Self {
        Self { no_color, cache_line_size }
    }

    pub fn format(&self, layouts: &[StructLayout]) -> String {
        let mut output = String::new();

        for (i, layout) in layouts.iter().enumerate() {
            if i > 0 {
                output.push_str("\n\n");
            }
            output.push_str(&self.format_struct(layout));
        }

        output
    }

    fn format_struct(&self, layout: &StructLayout) -> String {
        let mut output = String::new();

        let header = format!(
            "struct {} ({} bytes, {:.1}% padding, {} cache line{})",
            layout.name,
            layout.size,
            layout.metrics.padding_percentage,
            layout.metrics.cache_lines_spanned,
            if layout.metrics.cache_lines_spanned == 1 { "" } else { "s" }
        );

        if self.no_color {
            output.push_str(&header);
        } else {
            output.push_str(&header.bold().to_string());
        }
        output.push('\n');

        if let Some(ref loc) = layout.source_location {
            output.push_str(&format!("  defined at {}:{}\n", loc.file, loc.line));
        }
        output.push('\n');

        let mut table = Table::new();
        table.load_preset(UTF8_FULL_CONDENSED);
        table.set_header(vec!["Offset", "Size", "Type", "Field"]);

        let mut entries: Vec<TableEntry> = Vec::new();

        let mut padding_iter = layout.metrics.padding_holes.iter().peekable();

        for member in &layout.members {
            while let Some(hole) = padding_iter.peek() {
                if member.offset.map(|o| hole.offset < o).unwrap_or(false) {
                    let hole = padding_iter.next().unwrap();
                    entries.push(TableEntry::Padding { offset: hole.offset, size: hole.size });
                } else {
                    break;
                }
            }

            entries.push(TableEntry::Member {
                offset: member.offset,
                size: member.size,
                type_name: &member.type_name,
                name: &member.name,
                bit_offset: member.bit_offset,
                bit_size: member.bit_size,
            });
        }

        for hole in padding_iter {
            entries.push(TableEntry::Padding { offset: hole.offset, size: hole.size });
        }

        entries.sort_by_key(|e| match e {
            TableEntry::Member { offset, .. } => offset.unwrap_or(u64::MAX),
            TableEntry::Padding { offset, .. } => *offset,
        });

        let mut last_cache_line: Option<u64> = None;

        for entry in &entries {
            let offset = match entry {
                TableEntry::Member { offset: Some(o), .. } => Some(*o),
                TableEntry::Member { offset: None, .. } => None,
                TableEntry::Padding { offset, .. } => Some(*offset),
            };

            if let Some(off) = offset {
                let current_cache_line = off / self.cache_line_size as u64;
                if last_cache_line.is_some_and(|l| l != current_cache_line) {
                    let marker_offset = current_cache_line * self.cache_line_size as u64;
                    table.add_row(vec![
                        Cell::new(format!(
                            "--- cache line {} ({}) ---",
                            current_cache_line, marker_offset
                        ))
                        .set_alignment(CellAlignment::Center),
                        Cell::new(""),
                        Cell::new(""),
                        Cell::new(""),
                    ]);
                }
                last_cache_line = Some(current_cache_line);
            }

            match entry {
                TableEntry::Member { offset, size, type_name, name, bit_offset, bit_size } => {
                    let offset_str = match (offset, bit_offset) {
                        (Some(o), Some(bo)) => format!("{}:{}", o, bo),
                        (Some(o), None) => o.to_string(),
                        (None, Some(bo)) => format!("?:{}", bo),
                        (None, None) => "?".to_string(),
                    };
                    let size_str = match (size, bit_size) {
                        (_, Some(bs)) => format!("{}b", bs),
                        (Some(s), None) => s.to_string(),
                        (None, None) => "?".to_string(),
                    };
                    table.add_row(vec![
                        Cell::new(offset_str),
                        Cell::new(size_str),
                        Cell::new(type_name.to_string()),
                        Cell::new(name.to_string()),
                    ]);
                }
                TableEntry::Padding { offset, size } => {
                    let row = if self.no_color {
                        vec![
                            Cell::new(offset.to_string()),
                            Cell::new(format!("[{} bytes]", size)),
                            Cell::new("---"),
                            Cell::new("PAD"),
                        ]
                    } else {
                        vec![
                            Cell::new(offset.to_string()).fg(Color::Yellow),
                            Cell::new(format!("[{} bytes]", size)).fg(Color::Yellow),
                            Cell::new("---").fg(Color::Yellow),
                            Cell::new("PAD").fg(Color::Yellow),
                        ]
                    };
                    table.add_row(row);
                }
            }
        }

        output.push_str(&table.to_string());

        output.push_str(&format!(
            "\n\nSummary: {} useful bytes, {} padding bytes ({:.1}%), cache density: {:.1}%\n",
            layout.metrics.useful_size,
            layout.metrics.padding_bytes,
            layout.metrics.padding_percentage,
            layout.metrics.cache_line_density
        ));

        if let Some(ref fs) = layout.metrics.false_sharing {
            if !fs.warnings.is_empty() {
                let header = "\nPotential False Sharing:";
                if self.no_color {
                    output.push_str(header);
                } else {
                    output.push_str(&header.red().bold().to_string());
                }
                output.push('\n');

                for w in &fs.warnings {
                    let msg = format!(
                        "  - '{}' and '{}' share cache line {} (offset {}, {} bytes apart)",
                        w.member_a, w.member_b, w.cache_line, w.cache_line_offset, w.distance
                    );
                    if self.no_color {
                        output.push_str(&msg);
                    } else {
                        output.push_str(&msg.yellow().to_string());
                    }
                    output.push('\n');
                }
            }

            if !fs.atomic_members.is_empty() {
                let names: Vec<&str> = fs.atomic_members.iter().map(|m| m.name.as_str()).collect();
                output.push_str(&format!("\nAtomic members: {}\n", names.join(", ")));
            }
        }

        output
    }
}

enum TableEntry<'a> {
    Member {
        offset: Option<u64>,
        size: Option<u64>,
        type_name: &'a str,
        name: &'a str,
        bit_offset: Option<u64>,
        bit_size: Option<u64>,
    },
    Padding {
        offset: u64,
        size: u64,
    },
}
