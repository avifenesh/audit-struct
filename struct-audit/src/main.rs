use anyhow::{Context, Result};
use clap::Parser;
use std::path::Path;
use struct_audit::{
    BinaryData, Cli, Commands, DwarfContext, JsonFormatter, OutputFormat, TableFormatter,
    analyze_layout,
};

fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Commands::Inspect { binary, filter, output, no_color, cache_line, pretty } => {
            run_inspect(&binary, filter.as_deref(), output, no_color, cache_line, pretty)?;
        }
    }

    Ok(())
}

fn run_inspect(
    binary_path: &Path,
    filter: Option<&str>,
    output_format: OutputFormat,
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

    layouts.sort_by(|a, b| a.name.cmp(&b.name));

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
