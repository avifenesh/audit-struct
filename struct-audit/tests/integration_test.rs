use struct_audit::{BinaryData, DwarfContext, analyze_layout};

/// Get the path to the test fixture binary.
/// On macOS, debug info is in a separate dSYM bundle.
/// On Windows, binaries have .exe extension.
fn get_fixture_path() -> Option<std::path::PathBuf> {
    // Try dSYM path first (macOS)
    let dsym_path = std::path::Path::new(
        "tests/fixtures/bin/test_simple.dSYM/Contents/Resources/DWARF/test_simple",
    );
    if dsym_path.exists() {
        return Some(dsym_path.to_path_buf());
    }

    // Try Windows PE binary with .exe extension
    let exe_path = std::path::Path::new("tests/fixtures/bin/test_simple.exe");
    if exe_path.exists() {
        return Some(exe_path.to_path_buf());
    }

    // Fall back to direct binary (Linux with embedded debug info)
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

// ============================================================================
// Check command tests
// ============================================================================

fn create_temp_config(content: &str) -> std::path::PathBuf {
    use std::sync::atomic::{AtomicU64, Ordering};
    static COUNTER: AtomicU64 = AtomicU64::new(0);
    let temp_dir = std::env::temp_dir();
    let unique_id = COUNTER.fetch_add(1, Ordering::SeqCst);
    let config_path = temp_dir.join(format!(
        "struct-audit-test-{}-{}.yaml",
        std::process::id(),
        unique_id
    ));
    std::fs::write(&config_path, content).expect("Failed to write temp config");
    config_path
}

#[test]
fn test_check_budget_pass() {
    let path = match get_fixture_path() {
        Some(p) => p,
        None => return,
    };

    let config = create_temp_config(
        r#"
budgets:
  NoPadding:
    max_size: 100
    max_padding: 10
    max_padding_percent: 50.0
"#,
    );

    let output = std::process::Command::new("cargo")
        .args(["run", "--", "check", path.to_str().unwrap(), "--config", config.to_str().unwrap()])
        .output()
        .expect("Failed to run check command");

    std::fs::remove_file(&config).ok();

    assert!(
        output.status.success(),
        "Check should pass: {}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("All structs within budget"));
}

#[test]
fn test_check_budget_fail_size() {
    let path = match get_fixture_path() {
        Some(p) => p,
        None => return,
    };

    // InternalPadding is 16 bytes, so max_size: 10 should fail
    let config = create_temp_config(
        r#"
budgets:
  InternalPadding:
    max_size: 10
"#,
    );

    let output = std::process::Command::new("cargo")
        .args(["run", "--", "check", path.to_str().unwrap(), "--config", config.to_str().unwrap()])
        .output()
        .expect("Failed to run check command");

    std::fs::remove_file(&config).ok();

    assert!(!output.status.success(), "Check should fail for size violation");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("size") && stderr.contains("exceeds budget"),
        "Should mention size violation: {}",
        stderr
    );
}

#[test]
fn test_check_budget_fail_padding_percent() {
    let path = match get_fixture_path() {
        Some(p) => p,
        None => return,
    };

    // InternalPadding has 37.5% padding, so max_padding_percent: 5.0 should fail
    let config = create_temp_config(
        r#"
budgets:
  InternalPadding:
    max_padding_percent: 5.0
"#,
    );

    let output = std::process::Command::new("cargo")
        .args(["run", "--", "check", path.to_str().unwrap(), "--config", config.to_str().unwrap()])
        .output()
        .expect("Failed to run check command");

    std::fs::remove_file(&config).ok();

    assert!(!output.status.success(), "Check should fail for padding percent violation");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("padding") && stderr.contains("%"),
        "Should mention padding percent violation: {}",
        stderr
    );
}

#[test]
fn test_check_invalid_negative_percent() {
    let path = match get_fixture_path() {
        Some(p) => p,
        None => return,
    };

    let config = create_temp_config(
        r#"
budgets:
  NoPadding:
    max_padding_percent: -5.0
"#,
    );

    let output = std::process::Command::new("cargo")
        .args(["run", "--", "check", path.to_str().unwrap(), "--config", config.to_str().unwrap()])
        .output()
        .expect("Failed to run check command");

    std::fs::remove_file(&config).ok();

    assert!(!output.status.success(), "Check should fail for negative percent");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("negative") || stderr.contains("Invalid budget"),
        "Should reject negative percent: {}",
        stderr
    );
}

#[test]
fn test_check_invalid_over_100_percent() {
    let path = match get_fixture_path() {
        Some(p) => p,
        None => return,
    };

    let config = create_temp_config(
        r#"
budgets:
  NoPadding:
    max_padding_percent: 150.0
"#,
    );

    let output = std::process::Command::new("cargo")
        .args(["run", "--", "check", path.to_str().unwrap(), "--config", config.to_str().unwrap()])
        .output()
        .expect("Failed to run check command");

    std::fs::remove_file(&config).ok();

    assert!(!output.status.success(), "Check should fail for percent > 100");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("exceed 100") || stderr.contains("Invalid budget"),
        "Should reject percent > 100: {}",
        stderr
    );
}

#[test]
fn test_check_invalid_zero_size() {
    let path = match get_fixture_path() {
        Some(p) => p,
        None => return,
    };

    let config = create_temp_config(
        r#"
budgets:
  NoPadding:
    max_size: 0
"#,
    );

    let output = std::process::Command::new("cargo")
        .args(["run", "--", "check", path.to_str().unwrap(), "--config", config.to_str().unwrap()])
        .output()
        .expect("Failed to run check command");

    std::fs::remove_file(&config).ok();

    assert!(!output.status.success(), "Check should fail for max_size: 0");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("greater than 0") || stderr.contains("Invalid budget"),
        "Should reject max_size: 0: {}",
        stderr
    );
}
