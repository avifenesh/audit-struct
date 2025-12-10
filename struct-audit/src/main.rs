use anyhow::{Context, Result, bail};
use clap::Parser;
use std::path::Path;
use struct_audit::{
    BinaryData, Cli, Commands, DwarfContext, JsonFormatter, OutputFormat, SortField,
    TableFormatter, analyze_layout, diff_layouts,
};

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
        } => {
            run_inspect(
                &binary,
                filter.as_deref(),
                output,
                sort_by,
                top,
                min_padding,
                no_color,
                cache_line,
                pretty,
            )?;
        }
        Commands::Diff { old, new, filter, output, cache_line, fail_on_regression } => {
            let has_regression = run_diff(&old, &new, filter.as_deref(), output, cache_line)?;
            if fail_on_regression && has_regression {
                std::process::exit(1);
            }
        }
        Commands::Check { binary, config, cache_line } => {
            run_check(&binary, &config, cache_line)?;
        }
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn run_inspect(
    binary_path: &Path,
    filter: Option<&str>,
    output_format: OutputFormat,
    sort_by: SortField,
    top: Option<usize>,
    min_padding: Option<u64>,
    no_color: bool,
    cache_line_size: u32,
    pretty: bool,
) -> Result<()> {
    let binary = BinaryData::load(binary_path)
        .with_context(|| format!("Failed to load binary: {}", binary_path.display()))?;

    let loaded = binary.load_dwarf().context("Failed to load DWARF debug info")?;

    let dwarf = DwarfContext::new(&loaded);

    let mut layouts = dwarf.find_structs(filter).context("Failed to parse struct layouts")?;

    if layouts.is_empty() {
        if let Some(f) = filter {
            eprintln!("No structs found matching filter: {}", f);
        } else {
            eprintln!("No structs found in binary");
        }
        return Ok(());
    }

    for layout in &mut layouts {
        analyze_layout(layout, cache_line_size);
    }

    if let Some(min) = min_padding {
        layouts.retain(|l| l.metrics.padding_bytes >= min);
    }

    if layouts.is_empty() {
        eprintln!("No structs match the filter criteria");
        return Ok(());
    }

    match sort_by {
        SortField::Name => layouts.sort_by(|a, b| a.name.cmp(&b.name)),
        SortField::Size => layouts.sort_by(|a, b| b.size.cmp(&a.size)),
        SortField::Padding => {
            layouts.sort_by(|a, b| b.metrics.padding_bytes.cmp(&a.metrics.padding_bytes))
        }
        SortField::PaddingPct => layouts.sort_by(|a, b| {
            b.metrics
                .padding_percentage
                .partial_cmp(&a.metrics.padding_percentage)
                .unwrap_or(std::cmp::Ordering::Equal)
        }),
    }

    if let Some(n) = top {
        layouts.truncate(n);
    }

    let output_str = match output_format {
        OutputFormat::Table => {
            let formatter = TableFormatter::new(no_color, cache_line_size);
            formatter.format(&layouts)
        }
        OutputFormat::Json => {
            let formatter = JsonFormatter::new(pretty);
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
) -> Result<bool> {
    let old_binary = BinaryData::load(old_path)
        .with_context(|| format!("Failed to load old binary: {}", old_path.display()))?;
    let new_binary = BinaryData::load(new_path)
        .with_context(|| format!("Failed to load new binary: {}", new_path.display()))?;

    let old_loaded = old_binary.load_dwarf().context("Failed to load DWARF from old binary")?;
    let new_loaded = new_binary.load_dwarf().context("Failed to load DWARF from new binary")?;

    let old_dwarf = DwarfContext::new(&old_loaded);
    let new_dwarf = DwarfContext::new(&new_loaded);

    let mut old_layouts = old_dwarf.find_structs(filter)?;
    let mut new_layouts = new_dwarf.find_structs(filter)?;

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

fn print_diff_table(diff: &struct_audit::DiffResult) {
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
                    struct_audit::diff::MemberChangeKind::Added => "+".green(),
                    struct_audit::diff::MemberChangeKind::Removed => "-".red(),
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

fn run_check(binary_path: &Path, config_path: &Path, cache_line_size: u32) -> Result<()> {
    if !config_path.exists() {
        bail!(
            "Config file not found: {}\n\nCreate a .struct-audit.yaml with budget constraints:\n\n\
            budgets:\n  MyStruct:\n    max_size: 64\n    max_padding: 8\n    max_padding_percent: 10.0",
            config_path.display()
        );
    }

    let config_str = std::fs::read_to_string(config_path)
        .with_context(|| format!("Failed to read config: {}", config_path.display()))?;

    let config: Config = serde_yaml::from_str(&config_str)
        .with_context(|| format!("Failed to parse config: {}", config_path.display()))?;

    let binary = BinaryData::load(binary_path)
        .with_context(|| format!("Failed to load binary: {}", binary_path.display()))?;

    let loaded = binary.load_dwarf().context("Failed to load DWARF debug info")?;
    let dwarf = DwarfContext::new(&loaded);

    if config.budgets.is_empty() {
        eprintln!("Warning: No budget constraints defined in config file");
        return Ok(());
    }

    let mut layouts = dwarf.find_structs(None)?;
    for layout in &mut layouts {
        analyze_layout(layout, cache_line_size);
    }

    // Track which budgets were matched
    let layout_names: std::collections::HashSet<&str> =
        layouts.iter().map(|l| l.name.as_str()).collect();

    // Warn about budgets for non-existent structs
    for budget_name in config.budgets.keys() {
        if !layout_names.contains(budget_name.as_str()) {
            eprintln!(
                "Warning: Budget defined for '{}' but struct not found in binary",
                budget_name
            );
        }
    }

    let mut violations = Vec::new();

    for layout in &layouts {
        if let Some(budget) = config.budgets.get(&layout.name) {
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
            if let Some(max_pct) = budget.max_padding_percent
                && layout.metrics.padding_percentage > max_pct
            {
                violations.push(format!(
                    "{}: padding {:.1}% exceeds budget {:.1}% (+{:.1}%)",
                    layout.name,
                    layout.metrics.padding_percentage,
                    max_pct,
                    layout.metrics.padding_percentage - max_pct
                ));
            }
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
    budgets: std::collections::HashMap<String, Budget>,
}

#[derive(serde::Deserialize)]
struct Budget {
    max_size: Option<u64>,
    max_padding: Option<u64>,
    max_padding_percent: Option<f64>,
}
