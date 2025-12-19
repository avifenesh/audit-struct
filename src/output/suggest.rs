//! Output formatters for suggest command.

use crate::analysis::OptimizedLayout;
use colored::Colorize;
use comfy_table::{Cell, Color, Table, presets::UTF8_FULL_CONDENSED};
use serde::Serialize;

pub struct SuggestTableFormatter {
    no_color: bool,
}

impl SuggestTableFormatter {
    pub fn new(no_color: bool) -> Self {
        Self { no_color }
    }

    pub fn format(&self, suggestions: &[OptimizedLayout]) -> String {
        let mut output = String::new();

        for (i, suggestion) in suggestions.iter().enumerate() {
            if i > 0 {
                output.push_str("\n\n");
            }
            output.push_str(&self.format_suggestion(suggestion));
        }

        output
    }

    fn format_suggestion(&self, s: &OptimizedLayout) -> String {
        let mut output = String::new();

        // Header with savings summary
        let header = if s.savings_bytes > 0 {
            format!(
                "struct {} ({} bytes -> {} bytes, saves {} bytes / {:.1}%)",
                s.name, s.original_size, s.optimized_size, s.savings_bytes, s.savings_percent
            )
        } else {
            format!("struct {} ({} bytes, already optimal)", s.name, s.original_size)
        };

        if self.no_color {
            output.push_str(&header);
        } else if s.savings_bytes > 0 {
            output.push_str(&header.green().bold().to_string());
        } else {
            output.push_str(&header.bold().to_string());
        }
        output.push_str("\n\n");

        // Current layout
        output.push_str("Current layout:\n");
        output.push_str(&self.format_members_table(&s.original_members));
        output.push('\n');

        // Suggested layout (only if there are savings)
        if s.savings_bytes > 0 {
            output.push_str("\nSuggested layout:\n");
            output.push_str(&self.format_members_table_colored(&s.optimized_members));
            output.push('\n');
        }

        // Warnings for skipped members
        if !s.skipped_members.is_empty() {
            let warning = format!(
                "\nWarning: {} member(s) skipped due to missing size/offset: {}",
                s.skipped_members.len(),
                s.skipped_members.join(", ")
            );
            if self.no_color {
                output.push_str(&warning);
            } else {
                output.push_str(&warning.yellow().to_string());
            }
            output.push('\n');
        }

        // Note about bitfields
        if s.has_bitfields {
            let note = "\nNote: Bitfield members kept together in their storage units.";
            if self.no_color {
                output.push_str(note);
            } else {
                output.push_str(&note.cyan().to_string());
            }
            output.push('\n');
        }

        // FFI warning (always show for optimizable structs)
        if s.savings_bytes > 0 {
            let ffi_warning = "\nReordering may affect serialization/FFI compatibility";
            if self.no_color {
                output.push_str(ffi_warning);
            } else {
                output.push_str(&ffi_warning.yellow().to_string());
            }
            output.push('\n');
        }

        output
    }

    fn format_members_table(&self, members: &[crate::analysis::OptimizedMember]) -> String {
        let mut table = Table::new();
        table.load_preset(UTF8_FULL_CONDENSED);
        table.set_header(vec!["Offset", "Size", "Align", "Type", "Field"]);

        for m in members {
            table.add_row(vec![
                Cell::new(m.offset.to_string()),
                Cell::new(m.size.to_string()),
                Cell::new(m.alignment.to_string()),
                Cell::new(&m.type_name),
                Cell::new(&m.name),
            ]);
        }

        table.to_string()
    }

    fn format_members_table_colored(&self, members: &[crate::analysis::OptimizedMember]) -> String {
        let mut table = Table::new();
        table.load_preset(UTF8_FULL_CONDENSED);
        table.set_header(vec!["Offset", "Size", "Align", "Type", "Field"]);

        for m in members {
            let row = if self.no_color {
                vec![
                    Cell::new(m.offset.to_string()),
                    Cell::new(m.size.to_string()),
                    Cell::new(m.alignment.to_string()),
                    Cell::new(&m.type_name),
                    Cell::new(&m.name),
                ]
            } else {
                vec![
                    Cell::new(m.offset.to_string()).fg(Color::Green),
                    Cell::new(m.size.to_string()).fg(Color::Green),
                    Cell::new(m.alignment.to_string()).fg(Color::Green),
                    Cell::new(&m.type_name).fg(Color::Green),
                    Cell::new(&m.name).fg(Color::Green),
                ]
            };
            table.add_row(row);
        }

        table.to_string()
    }
}

#[derive(Serialize)]
struct SuggestJsonOutput<'a> {
    version: &'static str,
    suggestions: &'a [OptimizedLayout],
    summary: SuggestSummary,
}

#[derive(Serialize)]
struct SuggestSummary {
    total_structs: usize,
    optimizable_structs: usize,
    total_savings_bytes: u64,
}

pub struct SuggestJsonFormatter {
    pretty: bool,
}

impl SuggestJsonFormatter {
    pub fn new(pretty: bool) -> Self {
        Self { pretty }
    }

    pub fn format(&self, suggestions: &[OptimizedLayout]) -> String {
        let optimizable = suggestions.iter().filter(|s| s.savings_bytes > 0).count();
        let total_savings: u64 = suggestions.iter().map(|s| s.savings_bytes).sum();

        let output = SuggestJsonOutput {
            version: env!("CARGO_PKG_VERSION"),
            suggestions,
            summary: SuggestSummary {
                total_structs: suggestions.len(),
                optimizable_structs: optimizable,
                total_savings_bytes: total_savings,
            },
        };

        if self.pretty {
            serde_json::to_string_pretty(&output)
                .unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e))
        } else {
            serde_json::to_string(&output).unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn suggestion(name: &str, savings: u64) -> OptimizedLayout {
        OptimizedLayout {
            name: name.to_string(),
            original_size: 16,
            optimized_size: 16 - savings,
            savings_bytes: savings,
            savings_percent: if savings == 0 { 0.0 } else { (savings as f64 / 16.0) * 100.0 },
            struct_alignment: 8,
            original_members: Vec::new(),
            optimized_members: Vec::new(),
            skipped_members: Vec::new(),
            has_bitfields: false,
        }
    }

    #[test]
    fn suggest_table_includes_warning_and_ffi_note() {
        let mut s = suggestion("Foo", 4);
        s.skipped_members = vec!["missing".to_string()];
        s.has_bitfields = true;
        let formatter = SuggestTableFormatter::new(true);
        let out = formatter.format(&[s]);
        assert!(out.contains("Reordering may affect"));
        assert!(out.contains("Warning"));
        assert!(out.contains("Bitfield"));
    }

    #[test]
    fn suggest_table_handles_no_savings() {
        let formatter = SuggestTableFormatter::new(true);
        let out = formatter.format(&[suggestion("Bar", 0)]);
        assert!(out.contains("already optimal"));
    }

    #[test]
    fn suggest_json_summary_fields() {
        let formatter = SuggestJsonFormatter::new(true);
        let out = formatter.format(&[suggestion("A", 4), suggestion("B", 0)]);
        let parsed: serde_json::Value = serde_json::from_str(&out).expect("valid JSON");
        assert_eq!(parsed["summary"]["total_structs"], 2);
        assert_eq!(parsed["summary"]["optimizable_structs"], 1);
    }
}
