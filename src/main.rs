use anyhow::{Context, Result, bail};
use clap::Parser;
use layout_audit::{
    BinaryData, Cli, Commands, DwarfContext, JsonFormatter, OutputFormat, SortField,
    SuggestJsonFormatter, SuggestTableFormatter, TableFormatter, analyze_false_sharing,
    analyze_layout, diff_layouts, optimize_layout,
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

fn main() -> Result<()> {
    let cli = Cli::parse();

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
            let has_regression =
                run_diff(&old, &new, filter.as_deref(), output, cache_line, include_go_runtime)?;
            if fail_on_regression && has_regression {
                std::process::exit(1);
            }
        }
        Commands::Check { binary, config, cache_line, include_go_runtime } => {
            run_check(&binary, &config, cache_line, include_go_runtime)?;
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

    let mut violations = Vec::new();

    for layout in &layouts {
        if let Some((budget, pattern_idx)) = compiled.find_budget(&layout.name) {
            // Mark glob pattern as matched
            if let Some(idx) = pattern_idx {
                pattern_matched[idx] = true;
            }

            if let Some(max_size) = budget.max_size
                && layout.size > max_size
            {
                violations.push(format!(
                    "{}: size {} exceeds budget {} (+{} bytes)",
                    layout.name,
                    layout.size,
                    max_size,
                    layout.size - max_size
                ));
            }
            if let Some(max_padding) = budget.max_padding
                && layout.metrics.padding_bytes > max_padding
            {
                violations.push(format!(
                    "{}: padding {} exceeds budget {} (+{} bytes)",
                    layout.name,
                    layout.metrics.padding_bytes,
                    max_padding,
                    layout.metrics.padding_bytes - max_padding
                ));
            }
            if let Some(max_pct) = budget.max_padding_percent {
                const EPSILON: f64 = 1e-6;
                if layout.metrics.padding_percentage > max_pct + EPSILON {
                    violations.push(format!(
                        "{}: padding {:.1}% exceeds budget {:.1}% (+{:.1} percentage points)",
                        layout.name,
                        layout.metrics.padding_percentage,
                        max_pct,
                        layout.metrics.padding_percentage - max_pct
                    ));
                }
            }
            if let Some(max_fs) = budget.max_false_sharing_warnings {
                let fs = analyze_false_sharing(layout, cache_line_size);
                // Clamp to u32::MAX to prevent truncation on 64-bit platforms
                let warning_count = fs.warnings.len().min(u32::MAX as usize) as u32;
                if warning_count > max_fs {
                    violations.push(format!(
                        "{}: {} potential false sharing issue(s) exceeds limit of {}",
                        layout.name, warning_count, max_fs
                    ));
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

    if violations.is_empty() {
        println!("All structs within budget constraints");
        Ok(())
    } else {
        use colored::Colorize;
        eprintln!("{}", "Budget violations:".red().bold());
        for v in &violations {
            eprintln!("  {}", v);
        }
        bail!("Budget check failed: {} violation(s)", violations.len());
    }
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

    // Optimize each layout
    let mut suggestions: Vec<_> = layouts.iter().map(|l| optimize_layout(l, max_align)).collect();

    // Filter by minimum savings
    if let Some(min) = min_savings {
        suggestions.retain(|s| s.savings_bytes >= min);
    }

    if suggestions.is_empty() {
        eprintln!("No structs with optimization potential found");
        return Ok(());
    }

    // Sort by savings if requested
    if sort_by_savings {
        suggestions.sort_by(|a, b| b.savings_bytes.cmp(&a.savings_bytes));
    }

    let output_str = match output_format {
        OutputFormat::Table => {
            let formatter = SuggestTableFormatter::new(no_color);
            formatter.format(&suggestions)
        }
        OutputFormat::Json => {
            let formatter = SuggestJsonFormatter::new(pretty);
            formatter.format(&suggestions)
        }
    };

    println!("{}", output_str);

    Ok(())
}
