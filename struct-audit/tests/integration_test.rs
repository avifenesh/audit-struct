use struct_audit::{BinaryData, DwarfContext, analyze_layout};

/// Get the path to the test fixture binary.
/// On macOS, debug info is in a separate dSYM bundle.
fn get_fixture_path() -> Option<std::path::PathBuf> {
    // Try dSYM path first (macOS)
    let dsym_path = std::path::Path::new(
        "tests/fixtures/bin/test_simple.dSYM/Contents/Resources/DWARF/test_simple",
    );
    if dsym_path.exists() {
        return Some(dsym_path.to_path_buf());
    }

    // Fall back to direct binary (Linux/Windows with embedded debug info)
    let direct_path = std::path::Path::new("tests/fixtures/bin/test_simple");
    if direct_path.exists() {
        return Some(direct_path.to_path_buf());
    }

    None
}

#[test]
fn test_parse_simple_struct() {
    let path = match get_fixture_path() {
        Some(p) => p,
        None => {
            eprintln!(
                "Test fixture not compiled. Run: gcc -g -o tests/fixtures/bin/test_simple tests/fixtures/test_simple.c"
            );
            return;
        }
    };

    let binary = BinaryData::load(&path).expect("Failed to load binary");
    let loaded = binary.load_dwarf().expect("Failed to load DWARF");
    let dwarf = DwarfContext::new(&loaded);

    let mut layouts = dwarf.find_structs(Some("NoPadding")).expect("Failed to parse structs");

    assert_eq!(layouts.len(), 1);
    let layout = &mut layouts[0];

    analyze_layout(layout, 64);

    assert_eq!(layout.name, "NoPadding");
    assert_eq!(layout.size, 12);
    assert_eq!(layout.members.len(), 3);
    assert_eq!(layout.metrics.padding_bytes, 0);
}

#[test]
fn test_detect_padding() {
    let path = match get_fixture_path() {
        Some(p) => p,
        None => return,
    };

    let binary = BinaryData::load(&path).expect("Failed to load binary");
    let loaded = binary.load_dwarf().expect("Failed to load DWARF");
    let dwarf = DwarfContext::new(&loaded);

    let mut layouts = dwarf.find_structs(Some("InternalPadding")).expect("Failed to parse structs");

    assert_eq!(layouts.len(), 1);
    let layout = &mut layouts[0];

    analyze_layout(layout, 64);

    assert_eq!(layout.name, "InternalPadding");
    assert_eq!(layout.size, 16);
    assert!(layout.metrics.padding_bytes > 0);
    assert!(!layout.metrics.padding_holes.is_empty());
}

#[test]
fn test_tail_padding() {
    let path = match get_fixture_path() {
        Some(p) => p,
        None => return,
    };

    let binary = BinaryData::load(&path).expect("Failed to load binary");
    let loaded = binary.load_dwarf().expect("Failed to load DWARF");
    let dwarf = DwarfContext::new(&loaded);

    let mut layouts = dwarf.find_structs(Some("TailPadding")).expect("Failed to parse structs");

    assert_eq!(layouts.len(), 1);
    let layout = &mut layouts[0];

    analyze_layout(layout, 64);

    assert_eq!(layout.name, "TailPadding");
    assert_eq!(layout.size, 8);
    assert_eq!(layout.members.len(), 2);
    // Should have 3 bytes of tail padding after char b
    assert_eq!(layout.metrics.padding_bytes, 3);
}

#[test]
fn test_nested_struct() {
    let path = match get_fixture_path() {
        Some(p) => p,
        None => return,
    };

    let binary = BinaryData::load(&path).expect("Failed to load binary");
    let loaded = binary.load_dwarf().expect("Failed to load DWARF");
    let dwarf = DwarfContext::new(&loaded);

    let mut layouts = dwarf.find_structs(Some("Outer")).expect("Failed to parse structs");

    assert_eq!(layouts.len(), 1);
    let layout = &mut layouts[0];

    analyze_layout(layout, 64);

    assert_eq!(layout.name, "Outer");
    // char(1) + pad(3) + Inner(8) + char(1) + pad(3) = 16
    assert_eq!(layout.size, 16);
}

#[test]
fn test_cache_line_metrics() {
    let path = match get_fixture_path() {
        Some(p) => p,
        None => return,
    };

    let binary = BinaryData::load(&path).expect("Failed to load binary");
    let loaded = binary.load_dwarf().expect("Failed to load DWARF");
    let dwarf = DwarfContext::new(&loaded);

    let mut layouts = dwarf.find_structs(Some("NoPadding")).expect("Failed to parse structs");

    assert_eq!(layouts.len(), 1);
    let layout = &mut layouts[0];

    // Test with 64-byte cache lines
    analyze_layout(layout, 64);

    assert_eq!(layout.metrics.cache_lines_spanned, 1);
    assert!(layout.metrics.cache_line_density > 0.0);
    assert!(layout.metrics.cache_line_density <= 100.0);
}

#[test]
fn test_cli_json_output() {
    let path = match get_fixture_path() {
        Some(p) => p,
        None => return,
    };

    // Test that JSON output is valid
    let output = std::process::Command::new("cargo")
        .args([
            "run",
            "--",
            "inspect",
            path.to_str().unwrap(),
            "-o",
            "json",
            "--filter",
            "NoPadding",
        ])
        .output()
        .expect("Failed to run CLI");

    assert!(output.status.success(), "CLI failed: {:?}", String::from_utf8_lossy(&output.stderr));

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("Invalid JSON output");
    assert!(parsed.is_object(), "JSON root should be object");
    assert!(parsed["structs"].is_array(), "structs should be array");

    let structs = parsed["structs"].as_array().unwrap();
    assert_eq!(structs.len(), 1);
    assert_eq!(structs[0]["name"], "NoPadding");
}

#[test]
fn test_cli_sorting() {
    let path = match get_fixture_path() {
        Some(p) => p,
        None => return,
    };

    // Test sorting by padding
    let output = std::process::Command::new("cargo")
        .args([
            "run",
            "--",
            "inspect",
            path.to_str().unwrap(),
            "-o",
            "json",
            "--sort-by",
            "padding",
        ])
        .output()
        .expect("Failed to run CLI");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("Invalid JSON");
    let structs = parsed["structs"].as_array().unwrap();

    // Verify sorted in descending order by padding
    for i in 1..structs.len() {
        let prev_padding = structs[i - 1]["metrics"]["padding_bytes"].as_u64().unwrap();
        let curr_padding = structs[i]["metrics"]["padding_bytes"].as_u64().unwrap();
        assert!(prev_padding >= curr_padding, "Not sorted by padding");
    }
}

#[test]
fn test_cli_top_n() {
    let path = match get_fixture_path() {
        Some(p) => p,
        None => return,
    };

    // Test --top flag
    let output = std::process::Command::new("cargo")
        .args(["run", "--", "inspect", path.to_str().unwrap(), "-o", "json", "--top", "2"])
        .output()
        .expect("Failed to run CLI");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("Invalid JSON");
    let structs = parsed["structs"].as_array().unwrap();

    assert!(structs.len() <= 2, "Should return at most 2 structs");
}

#[test]
fn test_cli_min_padding_filter() {
    let path = match get_fixture_path() {
        Some(p) => p,
        None => return,
    };

    // Test --min-padding flag
    let output = std::process::Command::new("cargo")
        .args(["run", "--", "inspect", path.to_str().unwrap(), "-o", "json", "--min-padding", "1"])
        .output()
        .expect("Failed to run CLI");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("Invalid JSON");
    let structs = parsed["structs"].as_array().unwrap();

    // All returned structs should have at least 1 byte of padding
    for s in structs {
        let padding = s["metrics"]["padding_bytes"].as_u64().unwrap();
        assert!(padding >= 1, "Struct has less than min padding");
    }
}
