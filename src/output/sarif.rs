use crate::analysis::OptimizedLayout;
use crate::diff::DiffResult;
use crate::types::{SourceLocation, StructLayout};
use serde::Serialize;
use serde_json::{Value, json};
use std::collections::BTreeSet;

const SARIF_VERSION: &str = "2.1.0";
const SARIF_SCHEMA: &str = "https://schemastore.azurewebsites.net/schemas/json/sarif-2.1.0.json";
const TOOL_NAME: &str = "layout-audit";
const TOOL_URI: &str = "https://github.com/avifenesh/layout-audit";

const RULE_SIZE_INCREASE: &str = "LAYOUT-SIZE-INCREASE";
const RULE_PADDING_INCREASE: &str = "LAYOUT-PADDING-INCREASE";
const RULE_BUDGET_SIZE: &str = "LAYOUT-BUDGET-SIZE";
const RULE_BUDGET_PADDING: &str = "LAYOUT-BUDGET-PADDING";
const RULE_BUDGET_PADDING_PERCENT: &str = "LAYOUT-BUDGET-PADDING-PERCENT";
const RULE_BUDGET_FALSE_SHARING: &str = "LAYOUT-BUDGET-FALSE-SHARING";
const RULE_PADDING: &str = "LAYOUT-PADDING";
const RULE_FALSE_SHARING: &str = "LAYOUT-FALSE-SHARING";
const RULE_REORDER_SUGGESTION: &str = "LAYOUT-REORDER-SUGGESTION";

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckViolationKind {
    MaxSize,
    MaxPaddingBytes,
    MaxPaddingPercent,
    MaxFalseSharingWarnings,
}

#[derive(Debug, Clone, Serialize)]
pub struct CheckViolation {
    pub struct_name: String,
    pub kind: CheckViolationKind,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_location: Option<SourceLocation>,
}

pub struct SarifFormatter {
    tool_version: &'static str,
}

impl SarifFormatter {
    pub fn new() -> Self {
        Self { tool_version: env!("CARGO_PKG_VERSION") }
    }

    pub fn format_diff(&self, diff: &DiffResult, error_on_regression: bool) -> String {
        let mut results: Vec<Value> = Vec::new();
        let mut used_rules: BTreeSet<&'static str> = BTreeSet::new();
        let level = if error_on_regression { "error" } else { "warning" };

        for change in &diff.changed {
            if change.size_delta > 0 {
                used_rules.insert(RULE_SIZE_INCREASE);
                let message = format!(
                    "Struct {} size increased from {} to {} (+{} bytes)",
                    change.name, change.old_size, change.new_size, change.size_delta
                );
                results.push(make_result(
                    RULE_SIZE_INCREASE,
                    level,
                    message,
                    change.source_location.as_ref(),
                    Some(json!({
                        "struct": change.name,
                        "old_size": change.old_size,
                        "new_size": change.new_size,
                        "delta": change.size_delta,
                    })),
                ));
            }

            if change.padding_delta > 0 {
                used_rules.insert(RULE_PADDING_INCREASE);
                let message = format!(
                    "Struct {} padding increased from {} to {} (+{} bytes)",
                    change.name, change.old_padding, change.new_padding, change.padding_delta
                );
                results.push(make_result(
                    RULE_PADDING_INCREASE,
                    level,
                    message,
                    change.source_location.as_ref(),
                    Some(json!({
                        "struct": change.name,
                        "old_padding": change.old_padding,
                        "new_padding": change.new_padding,
                        "delta": change.padding_delta,
                    })),
                ));
            }
        }

        let rules = build_rules(&used_rules);
        render_sarif(self.tool_version, rules, results)
    }

    pub fn format_check(&self, violations: &[CheckViolation]) -> String {
        let mut results: Vec<Value> = Vec::new();
        let mut used_rules: BTreeSet<&'static str> = BTreeSet::new();

        for v in violations {
            let rule_id = rule_id_for_kind(v.kind);
            used_rules.insert(rule_id);
            results.push(make_result(
                rule_id,
                "error",
                v.message.clone(),
                v.source_location.as_ref(),
                Some(json!({ "struct": v.struct_name })),
            ));
        }

        let rules = build_rules(&used_rules);
        render_sarif(self.tool_version, rules, results)
    }

    pub fn format_inspect(&self, layouts: &[StructLayout]) -> String {
        let mut results: Vec<Value> = Vec::new();
        let mut used_rules: BTreeSet<&'static str> = BTreeSet::new();

        for layout in layouts {
            if layout.metrics.padding_bytes > 0 {
                used_rules.insert(RULE_PADDING);
                let message = format!(
                    "Struct {} has {} padding bytes ({:.1}% of {} bytes)",
                    layout.name,
                    layout.metrics.padding_bytes,
                    layout.metrics.padding_percentage,
                    layout.size
                );
                results.push(make_result(
                    RULE_PADDING,
                    "warning",
                    message,
                    layout.source_location.as_ref(),
                    Some(json!({
                        "struct": layout.name,
                        "size": layout.size,
                        "padding_bytes": layout.metrics.padding_bytes,
                        "padding_percent": layout.metrics.padding_percentage,
                        "cache_lines_spanned": layout.metrics.cache_lines_spanned,
                    })),
                ));
            }

            if let Some(fs) = layout.metrics.false_sharing.as_ref() {
                if !fs.warnings.is_empty() || !fs.spanning_warnings.is_empty() {
                    used_rules.insert(RULE_FALSE_SHARING);
                    let warning_count = fs.warnings.len();
                    let spanning_count = fs.spanning_warnings.len();
                    let message = format!(
                        "Struct {} has {} potential false sharing warning(s) and {} cache-line spanning atomic(s)",
                        layout.name, warning_count, spanning_count
                    );
                    results.push(make_result(
                        RULE_FALSE_SHARING,
                        "warning",
                        message,
                        layout.source_location.as_ref(),
                        Some(json!({
                            "struct": layout.name,
                            "false_sharing_warnings": warning_count,
                            "spanning_warnings": spanning_count,
                        })),
                    ));
                }
            }
        }

        let rules = build_rules(&used_rules);
        render_sarif(self.tool_version, rules, results)
    }

    pub fn format_suggest(
        &self,
        suggestions: &[OptimizedLayout],
        locations: &[Option<SourceLocation>],
    ) -> String {
        let mut results: Vec<Value> = Vec::new();
        let mut used_rules: BTreeSet<&'static str> = BTreeSet::new();

        for (idx, suggestion) in suggestions.iter().enumerate() {
            if suggestion.savings_bytes == 0 {
                continue;
            }
            let location = locations.get(idx).and_then(|loc| loc.as_ref());
            used_rules.insert(RULE_REORDER_SUGGESTION);
            let message = format!(
                "Struct {} can save {} bytes ({:.1}%) by reordering fields",
                suggestion.name, suggestion.savings_bytes, suggestion.savings_percent
            );
            results.push(make_result(
                RULE_REORDER_SUGGESTION,
                "note",
                message,
                location,
                Some(json!({
                    "struct": suggestion.name,
                    "original_size": suggestion.original_size,
                    "optimized_size": suggestion.optimized_size,
                    "savings_bytes": suggestion.savings_bytes,
                    "savings_percent": suggestion.savings_percent,
                })),
            ));
        }

        let rules = build_rules(&used_rules);
        render_sarif(self.tool_version, rules, results)
    }
}

impl Default for SarifFormatter {
    fn default() -> Self {
        Self::new()
    }
}

fn rule_id_for_kind(kind: CheckViolationKind) -> &'static str {
    match kind {
        CheckViolationKind::MaxSize => RULE_BUDGET_SIZE,
        CheckViolationKind::MaxPaddingBytes => RULE_BUDGET_PADDING,
        CheckViolationKind::MaxPaddingPercent => RULE_BUDGET_PADDING_PERCENT,
        CheckViolationKind::MaxFalseSharingWarnings => RULE_BUDGET_FALSE_SHARING,
    }
}

fn build_rules(rule_ids: &BTreeSet<&'static str>) -> Vec<Value> {
    rule_ids
        .iter()
        .map(|id| {
            let (name, short) = rule_metadata(id);
            json!({
                "id": id,
                "name": name,
                "shortDescription": { "text": short },
            })
        })
        .collect()
}

fn rule_metadata(rule_id: &str) -> (&'static str, &'static str) {
    match rule_id {
        RULE_SIZE_INCREASE => {
            ("Struct size increased", "Struct size increased relative to baseline")
        }
        RULE_PADDING_INCREASE => {
            ("Struct padding increased", "Struct padding increased relative to baseline")
        }
        RULE_BUDGET_SIZE => ("Budget: size", "Struct size exceeded budget"),
        RULE_BUDGET_PADDING => ("Budget: padding bytes", "Struct padding bytes exceeded budget"),
        RULE_BUDGET_PADDING_PERCENT => {
            ("Budget: padding percent", "Struct padding percentage exceeded budget")
        }
        RULE_BUDGET_FALSE_SHARING => {
            ("Budget: false sharing", "Struct false sharing warnings exceeded budget")
        }
        RULE_PADDING => ("Padding detected", "Struct contains padding bytes"),
        RULE_FALSE_SHARING => ("Potential false sharing", "Atomic members share cache lines"),
        RULE_REORDER_SUGGESTION => {
            ("Reorder suggestion", "Struct can be reordered to reduce padding")
        }
        _ => ("Layout issue", "Layout-audit reported an issue"),
    }
}

fn make_result(
    rule_id: &str,
    level: &str,
    message: String,
    source_location: Option<&SourceLocation>,
    properties: Option<Value>,
) -> Value {
    let mut result = json!({
        "ruleId": rule_id,
        "level": level,
        "message": { "text": message },
    });

    if let Some(location) = source_location {
        let loc = json!({
            "physicalLocation": {
                "artifactLocation": { "uri": location.file },
                "region": { "startLine": location.line },
            }
        });
        result["locations"] = json!([loc]);
    }

    if let Some(props) = properties {
        result["properties"] = props;
    }

    result
}

fn render_sarif(tool_version: &str, rules: Vec<Value>, results: Vec<Value>) -> String {
    let sarif = json!({
        "version": SARIF_VERSION,
        "$schema": SARIF_SCHEMA,
        "runs": [{
            "tool": {
                "driver": {
                    "name": TOOL_NAME,
                    "version": tool_version,
                    "informationUri": TOOL_URI,
                    "rules": rules,
                }
            },
            "results": results,
        }]
    });

    serde_json::to_string_pretty(&sarif).unwrap_or_else(|e| format!("{{\"error\": \"{}\"}}", e))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff::{DiffResult, MemberChange, MemberChangeKind, StructChange, StructSummary};
    use crate::types::{
        CacheLineSpanningWarning, FalseSharingAnalysis, FalseSharingWarning, LayoutMetrics,
        SourceLocation, StructLayout,
    };

    fn parse_sarif(s: &str) -> Value {
        serde_json::from_str(s).expect("valid SARIF JSON")
    }

    fn basic_layout(name: &str) -> StructLayout {
        let mut layout = StructLayout::new(name.to_string(), 16, Some(8));
        layout.metrics = LayoutMetrics::default();
        layout
    }

    #[test]
    fn diff_sarif_empty_results() {
        let formatter = SarifFormatter::new();
        let diff = DiffResult {
            added: Vec::new(),
            removed: Vec::new(),
            changed: Vec::new(),
            unchanged_count: 0,
        };
        let sarif = formatter.format_diff(&diff, false);
        let parsed = parse_sarif(&sarif);
        let results = parsed["runs"][0]["results"].as_array().unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn diff_sarif_includes_location_and_rules() {
        let formatter = SarifFormatter::new();
        let change = StructChange {
            name: "Foo".to_string(),
            old_size: 8,
            new_size: 16,
            size_delta: 8,
            old_padding: 0,
            new_padding: 4,
            padding_delta: 4,
            member_changes: vec![MemberChange {
                kind: MemberChangeKind::Added,
                name: "x".to_string(),
                details: "offset Some(8), size Some(4)".to_string(),
            }],
            source_location: Some(SourceLocation { file: "src/foo.c".to_string(), line: 10 }),
            old_source_location: None,
        };
        let diff = DiffResult {
            added: vec![StructSummary {
                name: "Bar".to_string(),
                size: 8,
                padding_bytes: 0,
                source_location: None,
            }],
            removed: Vec::new(),
            changed: vec![change],
            unchanged_count: 0,
        };

        let sarif = formatter.format_diff(&diff, true);
        let parsed = parse_sarif(&sarif);
        let results = parsed["runs"][0]["results"].as_array().unwrap();
        assert_eq!(results.len(), 2);
        for result in results {
            assert!(result["ruleId"].is_string());
            assert_eq!(result["level"], "error");
            let locations = result["locations"].as_array().unwrap();
            assert_eq!(locations[0]["physicalLocation"]["artifactLocation"]["uri"], "src/foo.c");
        }
    }

    #[test]
    fn check_sarif_empty() {
        let formatter = SarifFormatter::new();
        let sarif = formatter.format_check(&[]);
        let parsed = parse_sarif(&sarif);
        let results = parsed["runs"][0]["results"].as_array().unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn check_sarif_maps_rules() {
        let formatter = SarifFormatter::new();
        let violations = vec![
            CheckViolation {
                struct_name: "Foo".to_string(),
                kind: CheckViolationKind::MaxSize,
                message: "Foo: size 16 exceeds budget 8 (+8 bytes)".to_string(),
                source_location: Some(SourceLocation { file: "src/foo.c".to_string(), line: 5 }),
            },
            CheckViolation {
                struct_name: "Bar".to_string(),
                kind: CheckViolationKind::MaxPaddingPercent,
                message: "Bar: padding 50.0% exceeds budget 10.0% (+40.0 percentage points)"
                    .to_string(),
                source_location: None,
            },
        ];
        let sarif = formatter.format_check(&violations);
        let parsed = parse_sarif(&sarif);
        let results = parsed["runs"][0]["results"].as_array().unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0]["ruleId"], RULE_BUDGET_SIZE);
        assert_eq!(results[1]["ruleId"], RULE_BUDGET_PADDING_PERCENT);
    }

    #[test]
    fn inspect_sarif_padding_and_false_sharing() {
        let formatter = SarifFormatter::new();
        let mut layout = basic_layout("Foo");
        layout.metrics.padding_bytes = 4;
        layout.metrics.padding_percentage = 25.0;
        layout.metrics.cache_lines_spanned = 1;
        layout.source_location = Some(SourceLocation { file: "src/foo.c".to_string(), line: 3 });
        layout.metrics.false_sharing = Some(FalseSharingAnalysis {
            warnings: vec![FalseSharingWarning {
                member_a: "a".to_string(),
                member_b: "b".to_string(),
                cache_line: 0,
                gap_bytes: 0,
            }],
            spanning_warnings: vec![CacheLineSpanningWarning {
                member: "a".to_string(),
                type_name: "AtomicU64".to_string(),
                offset: 0,
                size: 8,
                start_cache_line: 0,
                end_cache_line: 1,
                lines_spanned: 2,
            }],
            atomic_members: Vec::new(),
        });

        let sarif = formatter.format_inspect(&[layout]);
        let parsed = parse_sarif(&sarif);
        let results = parsed["runs"][0]["results"].as_array().unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn suggest_sarif_skips_zero_savings() {
        let formatter = SarifFormatter::new();
        let no_savings = OptimizedLayout {
            name: "NoSavings".to_string(),
            original_size: 16,
            optimized_size: 16,
            savings_bytes: 0,
            savings_percent: 0.0,
            struct_alignment: 8,
            original_members: Vec::new(),
            optimized_members: Vec::new(),
            skipped_members: Vec::new(),
            has_bitfields: false,
        };
        let mut savings = no_savings.clone();
        savings.name = "Savings".to_string();
        savings.optimized_size = 12;
        savings.savings_bytes = 4;
        savings.savings_percent = 25.0;

        let sarif = formatter.format_suggest(
            &[no_savings, savings],
            &[None, Some(SourceLocation { file: "src/foo.c".to_string(), line: 12 })],
        );
        let parsed = parse_sarif(&sarif);
        let results = parsed["runs"][0]["results"].as_array().unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0]["ruleId"], RULE_REORDER_SUGGESTION);
    }
}
