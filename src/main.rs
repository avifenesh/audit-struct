use anyhow::{Context, Result, bail};
use clap::Parser;
use layout_audit::{
    BinaryData, CheckViolation, CheckViolationKind, Cli, Commands, DwarfContext, JsonFormatter,
    OutputFormat, SarifFormatter, SortField, SuggestJsonFormatter, SuggestTableFormatter,
    TableFormatter, analyze_false_sharing, analyze_layout, diff_layouts, optimize_layout,
};
use std::path::Path;

/// Configuration for the inspect command
struct InspectConfig<'a> {
    binary_path: &'a Path,
    filter: Option<&'a str>,
    output_format: OutputFormat,
    sort_by: SortField,
    top: Option<usize>,
    min_padding: Option<u64>,
    no_color: bool,
    cache_line_size: u32,
    pretty: bool,
    warn_false_sharing: bool,
    include_go_runtime: bool,
}

fn run_cli(cli: Cli) -> Result<()> {
    match cli.command {
        Commands::Inspect {
            binary,
            filter,
            output,
            sort_by,
            top,
            min_padding,
            no_color,
            cache_line,
            pretty,
            warn_false_sharing,
            include_go_runtime,
        } => {
            let config = InspectConfig {
                binary_path: &binary,
                filter: filter.as_deref(),
                output_format: output,
                sort_by,
                top,
                min_padding,
                no_color,
                cache_line_size: cache_line,
                pretty,
                warn_false_sharing,
                include_go_runtime,
            };
            run_inspect(&config)?;
        }
        Commands::Diff {
            old,
            new,
            filter,
            output,
            cache_line,
            fail_on_regression,
            include_go_runtime,
        } => {
            let has_regression = run_diff(
                &old,
                &new,
                filter.as_deref(),
                output,
                cache_line,
                fail_on_regression,
                include_go_runtime,
            )?;
            if fail_on_regression && has_regression {
                std::process::exit(1);
            }
        }
        Commands::Check { binary, config, output, cache_line, include_go_runtime } => {
            run_check(&binary, &config, output, cache_line, include_go_runtime)?;
        }
        Commands::Suggest {
            binary,
            filter,
            output,
            min_savings,
            cache_line,
            pretty,
            max_align,
            sort_by_savings,
            no_color,
            include_go_runtime,
        } => {
            run_suggest(
                &binary,
                filter.as_deref(),
                output,
                min_savings,
                cache_line,
                pretty,
                max_align,
                sort_by_savings,
                no_color,
                include_go_runtime,
            )?;
        }
    }

    Ok(())
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    run_cli(cli)
}

fn run_inspect(config: &InspectConfig<'_>) -> Result<()> {
    let binary = BinaryData::load(config.binary_path)
        .with_context(|| format!("Failed to load binary: {}", config.binary_path.display()))?;

    let loaded = binary.load_dwarf().context("Failed to load DWARF debug info")?;

    let dwarf = DwarfContext::new(&loaded);

    let mut layouts = dwarf
        .find_structs(config.filter, config.include_go_runtime)
        .context("Failed to parse struct layouts")?;

    if layouts.is_empty() {
        if let Some(f) = config.filter {
            eprintln!("No structs found matching filter: {}", f);
        } else {
            eprintln!("No structs found in binary");
        }
        return Ok(());
    }

    for layout in &mut layouts {
        analyze_layout(layout, config.cache_line_size);
        if config.warn_false_sharing {
            let fs_analysis = analyze_false_sharing(layout, config.cache_line_size);
            layout.metrics.false_sharing = Some(fs_analysis);
        }
    }

    if let Some(min) = config.min_padding {
        layouts.retain(|l| l.metrics.padding_bytes >= min);
    }

    if layouts.is_empty() {
        eprintln!("No structs match the filter criteria");
        return Ok(());
    }

    match config.sort_by {
        SortField::Name => layouts.sort_by(|a, b| a.name.cmp(&b.name)),
        SortField::Size => layouts.sort_by(|a, b| b.size.cmp(&a.size)),
        SortField::Padding => {
            layouts.sort_by(|a, b| b.metrics.padding_bytes.cmp(&a.metrics.padding_bytes))
        }
        SortField::PaddingPct => layouts.sort_by(|a, b| {
            match (a.metrics.padding_percentage.is_nan(), b.metrics.padding_percentage.is_nan()) {
                (true, true) => std::cmp::Ordering::Equal,
                (true, false) => std::cmp::Ordering::Greater,
                (false, true) => std::cmp::Ordering::Less,
                (false, false) => b
                    .metrics
                    .padding_percentage
                    .partial_cmp(&a.metrics.padding_percentage)
                    .unwrap_or(std::cmp::Ordering::Equal),
            }
        }),
    }

    if let Some(n) = config.top {
        layouts.truncate(n);
    }

    let output_str = match config.output_format {
        OutputFormat::Table => {
            let formatter = TableFormatter::new(config.no_color, config.cache_line_size);
            formatter.format(&layouts)
        }
        OutputFormat::Json => {
            let formatter = JsonFormatter::new(config.pretty);
            formatter.format(&layouts)
        }
        OutputFormat::Sarif => {
            let formatter = SarifFormatter::new();
            formatter.format_inspect(&layouts)
        }
    };

    println!("{}", output_str);

    Ok(())
}

fn run_diff(
    old_path: &Path,
    new_path: &Path,
    filter: Option<&str>,
    output_format: OutputFormat,
    cache_line_size: u32,
    fail_on_regression: bool,
    include_go_runtime: bool,
) -> Result<bool> {
    let old_binary = BinaryData::load(old_path)
        .with_context(|| format!("Failed to load old binary: {}", old_path.display()))?;
    let new_binary = BinaryData::load(new_path)
        .with_context(|| format!("Failed to load new binary: {}", new_path.display()))?;

    let old_loaded = old_binary.load_dwarf().context("Failed to load DWARF from old binary")?;
    let new_loaded = new_binary.load_dwarf().context("Failed to load DWARF from new binary")?;

    let old_dwarf = DwarfContext::new(&old_loaded);
    let new_dwarf = DwarfContext::new(&new_loaded);

    let mut old_layouts = old_dwarf.find_structs(filter, include_go_runtime)?;
    let mut new_layouts = new_dwarf.find_structs(filter, include_go_runtime)?;

    for layout in &mut old_layouts {
        analyze_layout(layout, cache_line_size);
    }
    for layout in &mut new_layouts {
        analyze_layout(layout, cache_line_size);
    }

    let diff = diff_layouts(&old_layouts, &new_layouts);

    match output_format {
        OutputFormat::Json => {
            println!("{}", serde_json::to_string_pretty(&diff)?);
        }
        OutputFormat::Table => {
            print_diff_table(&diff);
        }
        OutputFormat::Sarif => {
            let formatter = SarifFormatter::new();
            println!("{}", formatter.format_diff(&diff, fail_on_regression));
        }
    }

    Ok(diff.has_regressions())
}

fn print_diff_table(diff: &layout_audit::DiffResult) {
    use colored::Colorize;

    if !diff.has_changes() {
        println!("No changes detected.");
        return;
    }

    if !diff.removed.is_empty() {
        println!("{}", "Removed structs:".red().bold());
        for s in &diff.removed {
            println!("  - {} ({} bytes, {} padding)", s.name, s.size, s.padding_bytes);
        }
        println!();
    }

    if !diff.added.is_empty() {
        println!("{}", "Added structs:".green().bold());
        for s in &diff.added {
            println!("  + {} ({} bytes, {} padding)", s.name, s.size, s.padding_bytes);
        }
        println!();
    }

    if !diff.changed.is_empty() {
        println!("{}", "Changed structs:".yellow().bold());
        for c in &diff.changed {
            let size_indicator = match c.size_delta.cmp(&0) {
                std::cmp::Ordering::Greater => format!("+{}", c.size_delta).red().to_string(),
                std::cmp::Ordering::Less => format!("{}", c.size_delta).green().to_string(),
                std::cmp::Ordering::Equal => "0".to_string(),
            };
            let pad_indicator = match c.padding_delta.cmp(&0) {
                std::cmp::Ordering::Greater => format!("+{}", c.padding_delta).red().to_string(),
                std::cmp::Ordering::Less => format!("{}", c.padding_delta).green().to_string(),
                std::cmp::Ordering::Equal => "0".to_string(),
            };

            println!(
                "  ~ {} (size: {} -> {} [{}], padding: {} -> {} [{}])",
                c.name,
                c.old_size,
                c.new_size,
                size_indicator,
                c.old_padding,
                c.new_padding,
                pad_indicator
            );

            for mc in &c.member_changes {
                let prefix = match mc.kind {
                    layout_audit::diff::MemberChangeKind::Added => "+".green(),
                    layout_audit::diff::MemberChangeKind::Removed => "-".red(),
                    _ => "~".yellow(),
                };
                println!("      {} {}: {}", prefix, mc.name, mc.details);
            }
        }
        println!();
    }

    println!(
        "Summary: {} added, {} removed, {} changed, {} unchanged",
        diff.added.len(),
        diff.removed.len(),
        diff.changed.len(),
        diff.unchanged_count
    );
}

fn run_check(
    binary_path: &Path,
    config_path: &Path,
    output_format: OutputFormat,
    cache_line_size: u32,
    include_go_runtime: bool,
) -> Result<()> {
    if !config_path.exists() {
        bail!(
            "Config file not found: {}\n\nCreate a .layout-audit.yaml with budget constraints:\n\n\
            budgets:\n  MyStruct:\n    max_size: 64\n    max_padding: 8\n    max_padding_percent: 10.0\n\n\
            Glob patterns are supported:\n  \"*Padding\":\n    max_padding_percent: 15.0",
            config_path.display()
        );
    }

    let config_str = std::fs::read_to_string(config_path)
        .with_context(|| format!("Failed to read config: {}", config_path.display()))?;

    let config: Config = serde_yaml::from_str(&config_str)
        .with_context(|| format!("Failed to parse config: {}", config_path.display()))?;

    if config.budgets.is_empty() {
        eprintln!("Warning: No budget constraints defined in config file");
        return Ok(());
    }

    // Compile patterns (validates and separates exact matches from globs)
    let compiled = config.compile()?;

    let binary = BinaryData::load(binary_path)
        .with_context(|| format!("Failed to load binary: {}", binary_path.display()))?;

    let loaded = binary.load_dwarf().context("Failed to load DWARF debug info")?;
    let dwarf = DwarfContext::new(&loaded);

    let mut layouts = dwarf.find_structs(None, include_go_runtime)?;
    for layout in &mut layouts {
        analyze_layout(layout, cache_line_size);
    }

    let layout_names: std::collections::HashSet<&str> =
        layouts.iter().map(|l| l.name.as_str()).collect();

    // Warn about unmatched exact budget names
    for name in compiled.exact.keys() {
        if !layout_names.contains(name.as_str()) {
            eprintln!("Warning: Budget defined for '{}' but struct not found in binary", name);
        }
    }

    // Track which glob patterns matched at least one struct
    let mut pattern_matched = vec![false; compiled.patterns.len()];

    let mut violations: Vec<CheckViolation> = Vec::new();

    for layout in &layouts {
        if let Some((budget, pattern_idx)) = compiled.find_budget(&layout.name) {
            // Mark glob pattern as matched
            if let Some(idx) = pattern_idx {
                pattern_matched[idx] = true;
            }

            let source_location = layout.source_location.clone();
            if let Some(max_size) = budget.max_size
                && layout.size > max_size
            {
                violations.push(CheckViolation {
                    struct_name: layout.name.clone(),
                    kind: CheckViolationKind::MaxSize,
                    message: format!(
                        "{}: size {} exceeds budget {} (+{} bytes)",
                        layout.name,
                        layout.size,
                        max_size,
                        layout.size - max_size
                    ),
                    source_location: source_location.clone(),
                });
            }
            if let Some(max_padding) = budget.max_padding
                && layout.metrics.padding_bytes > max_padding
            {
                violations.push(CheckViolation {
                    struct_name: layout.name.clone(),
                    kind: CheckViolationKind::MaxPaddingBytes,
                    message: format!(
                        "{}: padding {} exceeds budget {} (+{} bytes)",
                        layout.name,
                        layout.metrics.padding_bytes,
                        max_padding,
                        layout.metrics.padding_bytes - max_padding
                    ),
                    source_location: source_location.clone(),
                });
            }
            if let Some(max_pct) = budget.max_padding_percent {
                const EPSILON: f64 = 1e-6;
                if layout.metrics.padding_percentage > max_pct + EPSILON {
                    violations.push(CheckViolation {
                        struct_name: layout.name.clone(),
                        kind: CheckViolationKind::MaxPaddingPercent,
                        message: format!(
                            "{}: padding {:.1}% exceeds budget {:.1}% (+{:.1} percentage points)",
                            layout.name,
                            layout.metrics.padding_percentage,
                            max_pct,
                            layout.metrics.padding_percentage - max_pct
                        ),
                        source_location: source_location.clone(),
                    });
                }
            }
            if let Some(max_fs) = budget.max_false_sharing_warnings {
                let fs = analyze_false_sharing(layout, cache_line_size);
                // Clamp to u32::MAX to prevent truncation on 64-bit platforms
                let warning_count = fs.warnings.len().min(u32::MAX as usize) as u32;
                if warning_count > max_fs {
                    violations.push(CheckViolation {
                        struct_name: layout.name.clone(),
                        kind: CheckViolationKind::MaxFalseSharingWarnings,
                        message: format!(
                            "{}: {} potential false sharing issue(s) exceeds limit of {}",
                            layout.name, warning_count, max_fs
                        ),
                        source_location: source_location.clone(),
                    });
                }
            }
        }
    }

    // Warn about glob patterns that matched nothing
    for (i, matched) in pattern_matched.iter().enumerate() {
        if !*matched {
            eprintln!(
                "Warning: Pattern '{}' did not match any structs",
                compiled.patterns[i].original_pattern
            );
        }
    }

    match output_format {
        OutputFormat::Table => {
            if violations.is_empty() {
                println!("All structs within budget constraints");
                Ok(())
            } else {
                use colored::Colorize;
                eprintln!("{}", "Budget violations:".red().bold());
                for v in &violations {
                    eprintln!("  {}", v.message);
                }
                bail!("Budget check failed: {} violation(s)", violations.len());
            }
        }
        OutputFormat::Json => {
            let output = CheckJsonOutput {
                version: env!("CARGO_PKG_VERSION"),
                violations: &violations,
                summary: CheckSummary { total_violations: violations.len() },
            };
            println!("{}", serde_json::to_string_pretty(&output)?);
            if violations.is_empty() {
                Ok(())
            } else {
                bail!("Budget check failed: {} violation(s)", violations.len());
            }
        }
        OutputFormat::Sarif => {
            let formatter = SarifFormatter::new();
            println!("{}", formatter.format_check(&violations));
            if violations.is_empty() {
                Ok(())
            } else {
                bail!("Budget check failed: {} violation(s)", violations.len());
            }
        }
    }
}

#[derive(serde::Serialize)]
struct CheckJsonOutput<'a> {
    version: &'static str,
    violations: &'a [CheckViolation],
    summary: CheckSummary,
}

#[derive(serde::Serialize)]
struct CheckSummary {
    total_violations: usize,
}

#[derive(serde::Deserialize)]
struct Config {
    #[serde(default)]
    budgets: indexmap::IndexMap<String, Budget>,
}

#[derive(serde::Deserialize, Clone)]
struct Budget {
    max_size: Option<u64>,
    max_padding: Option<u64>,
    max_padding_percent: Option<f64>,
    max_false_sharing_warnings: Option<u32>,
}

impl Budget {
    fn validate(&self, name: &str) -> Result<()> {
        if let Some(max_pct) = self.max_padding_percent {
            if !max_pct.is_finite() {
                bail!("Invalid budget for '{}': max_padding_percent must be a finite number", name);
            }
            if max_pct < 0.0 {
                bail!(
                    "Invalid budget for '{}': max_padding_percent cannot be negative (got {:.1})",
                    name,
                    max_pct
                );
            }
            if max_pct > 100.0 {
                bail!(
                    "Invalid budget for '{}': max_padding_percent cannot exceed 100 (got {:.1})",
                    name,
                    max_pct
                );
            }
        }
        if let Some(max_size) = self.max_size
            && max_size == 0
        {
            bail!("Invalid budget for '{}': max_size must be greater than 0", name);
        }
        Ok(())
    }
}

/// Check if a pattern string contains glob metacharacters
fn is_glob_pattern(s: &str) -> bool {
    s.contains('*') || s.contains('?') || s.contains('[') || s.contains('{')
}

/// Compiled budget patterns for efficient matching
struct CompiledBudgets {
    /// Exact name matches (O(1) lookup)
    exact: std::collections::HashMap<String, Budget>,
    /// Glob patterns in declaration order
    patterns: Vec<CompiledPattern>,
}

struct CompiledPattern {
    glob: globset::GlobMatcher,
    budget: Budget,
    original_pattern: String,
}

impl Config {
    /// Compile budget patterns for efficient matching.
    /// Separates exact matches from glob patterns.
    fn compile(&self) -> Result<CompiledBudgets> {
        use globset::GlobBuilder;

        let mut exact = std::collections::HashMap::new();
        let mut patterns = Vec::new();

        for (name, budget) in &self.budgets {
            if name.is_empty() {
                bail!("Empty budget pattern name is not allowed");
            }

            budget.validate(name)?;

            if is_glob_pattern(name) {
                let glob = GlobBuilder::new(name)
                    .literal_separator(false) // * matches ::
                    .build()
                    .with_context(|| format!("Invalid glob pattern: '{}'", name))?
                    .compile_matcher();

                patterns.push(CompiledPattern {
                    glob,
                    budget: budget.clone(),
                    original_pattern: name.clone(),
                });
            } else {
                exact.insert(name.clone(), budget.clone());
            }
        }

        Ok(CompiledBudgets { exact, patterns })
    }
}

impl CompiledBudgets {
    /// Find the budget for a struct name.
    /// Returns (budget, pattern_index) where pattern_index is Some if matched by a glob.
    fn find_budget(&self, struct_name: &str) -> Option<(&Budget, Option<usize>)> {
        // Exact match takes priority
        if let Some(budget) = self.exact.get(struct_name) {
            return Some((budget, None));
        }
        // First matching glob wins
        for (i, pattern) in self.patterns.iter().enumerate() {
            if pattern.glob.is_match(struct_name) {
                return Some((&pattern.budget, Some(i)));
            }
        }
        None
    }
}

#[allow(clippy::too_many_arguments)]
fn run_suggest(
    binary_path: &Path,
    filter: Option<&str>,
    output_format: OutputFormat,
    min_savings: Option<u64>,
    cache_line_size: u32,
    pretty: bool,
    max_align: u64,
    sort_by_savings: bool,
    no_color: bool,
    include_go_runtime: bool,
) -> Result<()> {
    let binary = BinaryData::load(binary_path)
        .with_context(|| format!("Failed to load binary: {}", binary_path.display()))?;

    let loaded = binary.load_dwarf().context("Failed to load DWARF debug info")?;
    let dwarf = DwarfContext::new(&loaded);

    let mut layouts =
        dwarf.find_structs(filter, include_go_runtime).context("Failed to parse struct layouts")?;

    if layouts.is_empty() {
        if let Some(f) = filter {
            eprintln!("No structs found matching filter: {}", f);
        } else {
            eprintln!("No structs found in binary");
        }
        return Ok(());
    }

    // Analyze layouts first (needed for metrics)
    for layout in &mut layouts {
        analyze_layout(layout, cache_line_size);
    }

    // Optimize each layout and keep source locations aligned
    let mut suggestions_with_locations: Vec<_> = layouts
        .iter()
        .map(|l| (optimize_layout(l, max_align), l.source_location.clone()))
        .collect();

    // Filter by minimum savings
    if let Some(min) = min_savings {
        suggestions_with_locations.retain(|(s, _)| s.savings_bytes >= min);
    }

    if suggestions_with_locations.is_empty() {
        eprintln!("No structs with optimization potential found");
        return Ok(());
    }

    // Sort by savings if requested
    if sort_by_savings {
        suggestions_with_locations.sort_by(|(a, _), (b, _)| b.savings_bytes.cmp(&a.savings_bytes));
    }

    let (suggestions, locations): (Vec<_>, Vec<_>) = suggestions_with_locations.into_iter().unzip();

    let output_str = match output_format {
        OutputFormat::Table => {
            let formatter = SuggestTableFormatter::new(no_color);
            formatter.format(&suggestions)
        }
        OutputFormat::Json => {
            let formatter = SuggestJsonFormatter::new(pretty);
            formatter.format(&suggestions)
        }
        OutputFormat::Sarif => {
            let formatter = SarifFormatter::new();
            formatter.format_suggest(&suggestions, &locations)
        }
    };

    println!("{}", output_str);

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

    fn find_fixture_path(name: &str) -> Option<PathBuf> {
        let base = Path::new("tests/fixtures/bin");
        let dsym_path = base.join(format!("{}.dSYM/Contents/Resources/DWARF/{}", name, name));
        if dsym_path.exists() {
            return Some(dsym_path);
        }

        let exe_path = base.join(format!("{}.exe", name));
        if exe_path.exists() {
            return Some(exe_path);
        }

        let direct_path = base.join(name);
        if direct_path.exists() {
            return Some(direct_path);
        }

        None
    }

    fn create_temp_config(content: &str) -> PathBuf {
        use std::sync::atomic::{AtomicU64, Ordering};
        static COUNTER: AtomicU64 = AtomicU64::new(0);
        let temp_dir = std::env::temp_dir();
        let unique_id = COUNTER.fetch_add(1, Ordering::SeqCst);
        let config_path = temp_dir.join(format!(
            "layout-audit-main-test-{}-{}.yaml",
            std::process::id(),
            unique_id
        ));
        std::fs::write(&config_path, content).expect("Failed to write temp config");
        config_path
    }

    #[test]
    fn run_inspect_outputs() {
        let path = match find_fixture_path("test_simple") {
            Some(p) => p,
            None => return,
        };

        let base = InspectConfig {
            binary_path: &path,
            filter: Some("Padding"),
            output_format: OutputFormat::Table,
            sort_by: SortField::Name,
            top: Some(1),
            min_padding: None,
            no_color: true,
            cache_line_size: 64,
            pretty: true,
            warn_false_sharing: true,
            include_go_runtime: false,
        };

        run_inspect(&base).expect("inspect table");
        let json_cfg = InspectConfig { output_format: OutputFormat::Json, ..base };
        run_inspect(&json_cfg).expect("inspect json");
        let sarif_cfg = InspectConfig { output_format: OutputFormat::Sarif, ..base };
        run_inspect(&sarif_cfg).expect("inspect sarif");
    }

    #[test]
    fn run_diff_outputs() {
        let path = match find_fixture_path("test_simple") {
            Some(p) => p,
            None => return,
        };

        run_diff(&path, &path, None, OutputFormat::Table, 64, false, false).expect("diff table");
        run_diff(&path, &path, None, OutputFormat::Json, 64, false, false).expect("diff json");
        run_diff(&path, &path, None, OutputFormat::Sarif, 64, false, false).expect("diff sarif");
    }

    #[test]
    fn run_check_outputs() {
        let path = match find_fixture_path("test_simple") {
            Some(p) => p,
            None => return,
        };

        let config = create_temp_config(
            r#"
budgets:
  NoPadding:
    max_size: 100
    max_padding: 20
    max_padding_percent: 80.0
"#,
        );

        run_check(&path, &config, OutputFormat::Table, 64, false).expect("check table");
        run_check(&path, &config, OutputFormat::Json, 64, false).expect("check json");
        run_check(&path, &config, OutputFormat::Sarif, 64, false).expect("check sarif");

        std::fs::remove_file(&config).ok();
    }

    #[test]
    fn run_check_failure_path() {
        let path = match find_fixture_path("test_simple") {
            Some(p) => p,
            None => return,
        };

        let config = create_temp_config(
            r#"
budgets:
  InternalPadding:
    max_size: 10
"#,
        );

        let result = run_check(&path, &config, OutputFormat::Table, 64, false);
        std::fs::remove_file(&config).ok();
        assert!(result.is_err());
    }

    #[test]
    fn run_check_json_failure_path() {
        let path = match find_fixture_path("test_simple") {
            Some(p) => p,
            None => return,
        };

        let config = create_temp_config(
            r#"
budgets:
  InternalPadding:
    max_size: 1
"#,
        );

        let result = run_check(&path, &config, OutputFormat::Json, 64, false);
        std::fs::remove_file(&config).ok();
        assert!(result.is_err());
    }

    #[test]
    fn run_check_sarif_failure_path() {
        let path = match find_fixture_path("test_simple") {
            Some(p) => p,
            None => return,
        };

        let config = create_temp_config(
            r#"
budgets:
  InternalPadding:
    max_size: 1
"#,
        );

        let result = run_check(&path, &config, OutputFormat::Sarif, 64, false);
        std::fs::remove_file(&config).ok();
        assert!(result.is_err());
    }

    #[test]
    fn run_check_false_sharing_violation() {
        let path = match find_fixture_path("test_simple") {
            Some(p) => p,
            None => return,
        };

        let binary = match BinaryData::load(&path) {
            Ok(b) => b,
            Err(_) => return,
        };
        let loaded = match binary.load_dwarf() {
            Ok(l) => l,
            Err(_) => return,
        };
        let dwarf = DwarfContext::new(&loaded);
        let mut layouts = match dwarf.find_structs(Some("WithAtomics"), false) {
            Ok(l) => l,
            Err(_) => return,
        };
        if layouts.is_empty() {
            return;
        }
        let layout = &mut layouts[0];
        analyze_layout(layout, 64);
        let fs = analyze_false_sharing(layout, 64);
        if fs.warnings.is_empty() {
            return;
        }

        let config = create_temp_config(
            r#"
budgets:
  WithAtomics:
    max_false_sharing_warnings: 0
"#,
        );

        let result = run_check(&path, &config, OutputFormat::Table, 64, false);
        std::fs::remove_file(&config).ok();
        assert!(result.is_err());
    }

    #[test]
    fn run_suggest_outputs() {
        let path = match find_fixture_path("test_simple") {
            Some(p) => p,
            None => return,
        };

        run_suggest(&path, None, OutputFormat::Table, Some(1), 64, true, 8, false, true, false)
            .expect("suggest table");

        run_suggest(&path, None, OutputFormat::Json, Some(1), 64, true, 8, false, true, false)
            .expect("suggest json");

        run_suggest(&path, None, OutputFormat::Sarif, Some(1), 64, true, 8, false, true, false)
            .expect("suggest sarif");
    }

    #[test]
    fn run_inspect_no_matches() {
        let path = match find_fixture_path("test_simple") {
            Some(p) => p,
            None => return,
        };

        let cfg = InspectConfig {
            binary_path: &path,
            filter: Some("DoesNotExist"),
            output_format: OutputFormat::Table,
            sort_by: SortField::Name,
            top: None,
            min_padding: None,
            no_color: true,
            cache_line_size: 64,
            pretty: false,
            warn_false_sharing: false,
            include_go_runtime: false,
        };

        run_inspect(&cfg).expect("inspect no matches");
    }

    #[test]
    fn run_inspect_min_padding_filters_all() {
        let path = match find_fixture_path("test_simple") {
            Some(p) => p,
            None => return,
        };

        let cfg = InspectConfig {
            binary_path: &path,
            filter: None,
            output_format: OutputFormat::Table,
            sort_by: SortField::PaddingPct,
            top: None,
            min_padding: Some(10_000),
            no_color: true,
            cache_line_size: 64,
            pretty: false,
            warn_false_sharing: false,
            include_go_runtime: false,
        };

        run_inspect(&cfg).expect("inspect min padding");
    }

    #[test]
    fn run_diff_with_changes_table() {
        let old_path = match find_fixture_path("test_simple") {
            Some(p) => p,
            None => return,
        };
        let new_path = match find_fixture_path("test_modified") {
            Some(p) => p,
            None => return,
        };

        run_diff(&old_path, &new_path, None, OutputFormat::Table, 64, false, false)
            .expect("diff table changes");
    }

    #[test]
    fn run_check_missing_config_path() {
        let path = match find_fixture_path("test_simple") {
            Some(p) => p,
            None => return,
        };

        let missing = Path::new("tests/fixtures/does-not-exist.yaml");
        let result = run_check(&path, missing, OutputFormat::Table, 64, false);
        assert!(result.is_err());
    }

    #[test]
    fn config_compile_invalid_patterns() {
        let cfg = Config {
            budgets: [(
                "".to_string(),
                Budget {
                    max_size: Some(1),
                    max_padding: None,
                    max_padding_percent: None,
                    max_false_sharing_warnings: None,
                },
            )]
            .into_iter()
            .collect(),
        };

        assert!(cfg.compile().is_err());

        let cfg = Config {
            budgets: [(
                "[invalid".to_string(),
                Budget {
                    max_size: Some(1),
                    max_padding: None,
                    max_padding_percent: None,
                    max_false_sharing_warnings: None,
                },
            )]
            .into_iter()
            .collect(),
        };

        assert!(cfg.compile().is_err());
    }

    #[test]
    fn budget_validate_rejects_invalid_percent() {
        let budget = Budget {
            max_size: None,
            max_padding: None,
            max_padding_percent: Some(200.0),
            max_false_sharing_warnings: None,
        };
        assert!(budget.validate("X").is_err());
    }

    #[test]
    fn run_check_warnings_for_unmatched_patterns() {
        let path = match find_fixture_path("test_simple") {
            Some(p) => p,
            None => return,
        };

        let config = create_temp_config(
            r#"
budgets:
  DoesNotExist:
    max_size: 999
  "NoMatch*":
    max_padding: 999
"#,
        );

        run_check(&path, &config, OutputFormat::Table, 64, false).expect("check warnings");
        std::fs::remove_file(&config).ok();
    }

    #[test]
    fn run_check_empty_budgets() {
        let path = match find_fixture_path("test_simple") {
            Some(p) => p,
            None => return,
        };

        let config = create_temp_config("budgets: {}");
        run_check(&path, &config, OutputFormat::Table, 64, false).expect("check empty budgets");
        std::fs::remove_file(&config).ok();
    }

    #[test]
    fn run_suggest_sort_by_savings_branch() {
        let path = match find_fixture_path("test_simple") {
            Some(p) => p,
            None => return,
        };

        run_suggest(&path, None, OutputFormat::Table, None, 64, true, 8, true, true, false)
            .expect("suggest sorted");
    }

    #[test]
    fn run_suggest_no_savings_path() {
        let path = match find_fixture_path("test_simple") {
            Some(p) => p,
            None => return,
        };

        run_suggest(
            &path,
            None,
            OutputFormat::Table,
            Some(10_000),
            64,
            true,
            8,
            false,
            true,
            false,
        )
        .expect("suggest no savings");
    }

    #[test]
    fn run_inspect_sort_variants() {
        let path = match find_fixture_path("test_simple") {
            Some(p) => p,
            None => return,
        };

        let cfg = InspectConfig {
            binary_path: &path,
            filter: Some("Padding"),
            output_format: OutputFormat::Table,
            sort_by: SortField::Size,
            top: None,
            min_padding: None,
            no_color: true,
            cache_line_size: 64,
            pretty: false,
            warn_false_sharing: false,
            include_go_runtime: false,
        };
        run_inspect(&cfg).expect("inspect size sort");

        let cfg = InspectConfig { sort_by: SortField::Padding, ..cfg };
        run_inspect(&cfg).expect("inspect padding sort");
    }

    #[test]
    fn glob_helpers_and_budget_lookup() {
        assert!(!is_glob_pattern("PlainName"));
        assert!(is_glob_pattern("*Padding"));

        let cfg = Config {
            budgets: [
                (
                    "Exact".to_string(),
                    Budget {
                        max_size: Some(1),
                        max_padding: None,
                        max_padding_percent: None,
                        max_false_sharing_warnings: None,
                    },
                ),
                (
                    "Glob*".to_string(),
                    Budget {
                        max_size: Some(2),
                        max_padding: None,
                        max_padding_percent: None,
                        max_false_sharing_warnings: None,
                    },
                ),
            ]
            .into_iter()
            .collect(),
        };

        let compiled = cfg.compile().expect("compile budgets");
        let exact = compiled.find_budget("Exact");
        assert!(exact.is_some());
        let glob = compiled.find_budget("GlobName");
        assert!(glob.is_some());
    }

    #[test]
    fn run_cli_dispatches_commands() {
        let path = match find_fixture_path("test_simple") {
            Some(p) => p,
            None => return,
        };

        let inspect = Cli {
            command: Commands::Inspect {
                binary: path.clone(),
                filter: Some("Padding".to_string()),
                output: OutputFormat::Table,
                sort_by: SortField::Name,
                top: Some(1),
                min_padding: None,
                no_color: true,
                cache_line: 64,
                pretty: false,
                warn_false_sharing: false,
                include_go_runtime: false,
            },
        };
        run_cli(inspect).expect("cli inspect");

        let diff = Cli {
            command: Commands::Diff {
                old: path.clone(),
                new: path.clone(),
                filter: None,
                output: OutputFormat::Json,
                cache_line: 64,
                fail_on_regression: false,
                include_go_runtime: false,
            },
        };
        run_cli(diff).expect("cli diff");

        let config = create_temp_config("budgets: {}");
        let check = Cli {
            command: Commands::Check {
                binary: path.clone(),
                config: config.clone(),
                output: OutputFormat::Table,
                cache_line: 64,
                include_go_runtime: false,
            },
        };
        run_cli(check).expect("cli check");
        std::fs::remove_file(&config).ok();

        let suggest = Cli {
            command: Commands::Suggest {
                binary: path,
                filter: None,
                output: OutputFormat::Json,
                min_savings: None,
                cache_line: 64,
                pretty: false,
                max_align: 8,
                sort_by_savings: false,
                no_color: true,
                include_go_runtime: false,
            },
        };
        run_cli(suggest).expect("cli suggest");
    }
}
