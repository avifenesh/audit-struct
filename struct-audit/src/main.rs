use anyhow::{Context, Result};
use clap::Parser;
use std::path::Path;
use struct_audit::{
    BinaryData, Cli, Commands, DwarfContext, JsonFormatter, OutputFormat, SortField,
    TableFormatter, analyze_layout,
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
