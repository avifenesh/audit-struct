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
    let config_path =
        temp_dir.join(format!("struct-audit-test-{}-{}.yaml", std::process::id(), unique_id));
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

// ============================================================================
// Array member tests
// ============================================================================

#[test]
fn test_array_member_size() {
    let path = match get_fixture_path() {
        Some(p) => p,
        None => return,
    };

    let binary = BinaryData::load(&path).expect("Failed to load binary");
    let loaded = binary.load_dwarf().expect("Failed to load DWARF");
    let dwarf = DwarfContext::new(&loaded);

    let mut layouts = dwarf.find_structs(Some("WithArray")).expect("Failed to parse structs");

    if layouts.is_empty() {
        // WithArray may not exist in all fixtures, skip test
        return;
    }

    let layout = &mut layouts[0];
    analyze_layout(layout, 64);

    // Find the array member
    let array_member = layout.members.iter().find(|m| m.type_name.contains('['));
    if let Some(member) = array_member {
        // Array members should have a known size, not None
        assert!(member.size.is_some(), "Array member should have size: {:?}", member);
        assert!(member.size.unwrap() > 0, "Array member size should be > 0");
    }
}

// ============================================================================
// Diff command tests
// ============================================================================

#[test]
fn test_diff_identical_binaries() {
    let path = match get_fixture_path() {
        Some(p) => p,
        None => return,
    };

    let output = std::process::Command::new("cargo")
        .args(["run", "--", "diff", path.to_str().unwrap(), path.to_str().unwrap(), "-o", "json"])
        .output()
        .expect("Failed to run diff command");

    assert!(
        output.status.success(),
        "Diff should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("Invalid JSON output");

    // Identical binaries should have no changes
    let added = parsed["added"].as_array().unwrap();
    let removed = parsed["removed"].as_array().unwrap();
    let changed = parsed["changed"].as_array().unwrap();
    assert!(
        added.is_empty() && removed.is_empty() && changed.is_empty(),
        "Identical binaries should have no diff changes, got: added={}, removed={}, changed={}",
        added.len(),
        removed.len(),
        changed.len()
    );
}

#[test]
fn test_diff_filter() {
    let path = match get_fixture_path() {
        Some(p) => p,
        None => return,
    };

    let output = std::process::Command::new("cargo")
        .args([
            "run",
            "--",
            "diff",
            path.to_str().unwrap(),
            path.to_str().unwrap(),
            "--filter",
            "NoPadding",
            "-o",
            "json",
        ])
        .output()
        .expect("Failed to run diff command");

    assert!(
        output.status.success(),
        "Diff with filter should succeed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

/// Get path to the modified test fixture for diff testing
fn get_modified_fixture_path() -> Option<std::path::PathBuf> {
    let dsym_path = std::path::Path::new(
        "tests/fixtures/bin/test_modified.dSYM/Contents/Resources/DWARF/test_modified",
    );
    if dsym_path.exists() {
        return Some(dsym_path.to_path_buf());
    }

    let exe_path = std::path::Path::new("tests/fixtures/bin/test_modified.exe");
    if exe_path.exists() {
        return Some(exe_path.to_path_buf());
    }

    let direct_path = std::path::Path::new("tests/fixtures/bin/test_modified");
    if direct_path.exists() {
        return Some(direct_path.to_path_buf());
    }

    None
}

#[test]
fn test_diff_detects_added_structs() {
    let old_path = match get_fixture_path() {
        Some(p) => p,
        None => return,
    };
    let new_path = match get_modified_fixture_path() {
        Some(p) => p,
        None => return,
    };

    let output = std::process::Command::new("cargo")
        .args([
            "run",
            "--",
            "diff",
            old_path.to_str().unwrap(),
            new_path.to_str().unwrap(),
            "-o",
            "json",
        ])
        .output()
        .expect("Failed to run diff command");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("Invalid JSON");

    let added = parsed["added"].as_array().unwrap();
    assert!(!added.is_empty(), "Should detect added structs");
    assert!(added.iter().any(|s| s["name"] == "NewStruct"), "Should detect NewStruct as added");
}

#[test]
fn test_diff_detects_removed_structs() {
    let old_path = match get_fixture_path() {
        Some(p) => p,
        None => return,
    };
    let new_path = match get_modified_fixture_path() {
        Some(p) => p,
        None => return,
    };

    let output = std::process::Command::new("cargo")
        .args([
            "run",
            "--",
            "diff",
            old_path.to_str().unwrap(),
            new_path.to_str().unwrap(),
            "-o",
            "json",
        ])
        .output()
        .expect("Failed to run diff command");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("Invalid JSON");

    let removed = parsed["removed"].as_array().unwrap();
    assert!(!removed.is_empty(), "Should detect removed structs");
    assert!(
        removed.iter().any(|s| s["name"] == "Inner" || s["name"] == "Outer"),
        "Should detect Inner or Outer as removed"
    );
}

#[test]
fn test_diff_detects_changed_structs() {
    let old_path = match get_fixture_path() {
        Some(p) => p,
        None => return,
    };
    let new_path = match get_modified_fixture_path() {
        Some(p) => p,
        None => return,
    };

    let output = std::process::Command::new("cargo")
        .args([
            "run",
            "--",
            "diff",
            old_path.to_str().unwrap(),
            new_path.to_str().unwrap(),
            "-o",
            "json",
        ])
        .output()
        .expect("Failed to run diff command");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("Invalid JSON");

    let changed = parsed["changed"].as_array().unwrap();
    assert!(!changed.is_empty(), "Should detect changed structs");

    // NoPadding should show size increase (added field d)
    let no_padding = changed.iter().find(|s| s["name"] == "NoPadding");
    assert!(no_padding.is_some(), "NoPadding should be in changed list");
    let np = no_padding.unwrap();
    assert_eq!(np["size_delta"], 4, "NoPadding should have grown by 4 bytes");
}

#[test]
fn test_diff_fail_on_regression() {
    let old_path = match get_fixture_path() {
        Some(p) => p,
        None => return,
    };
    let new_path = match get_modified_fixture_path() {
        Some(p) => p,
        None => return,
    };

    // With --fail-on-regression, should fail if structs grew
    let output = std::process::Command::new("cargo")
        .args([
            "run",
            "--",
            "diff",
            old_path.to_str().unwrap(),
            new_path.to_str().unwrap(),
            "--fail-on-regression",
        ])
        .output()
        .expect("Failed to run diff command");

    // NoPadding grew from 12 to 16 bytes, so this should fail
    assert!(!output.status.success(), "Should fail with --fail-on-regression when structs grow");
}

#[test]
fn test_check_budget_fail_max_padding_bytes() {
    let path = match get_fixture_path() {
        Some(p) => p,
        None => return,
    };

    // InternalPadding has 6 bytes of padding, so max_padding: 2 should fail
    let config = create_temp_config(
        r#"
budgets:
  InternalPadding:
    max_padding: 2
"#,
    );

    let output = std::process::Command::new("cargo")
        .args(["run", "--", "check", path.to_str().unwrap(), "--config", config.to_str().unwrap()])
        .output()
        .expect("Failed to run check command");

    std::fs::remove_file(&config).ok();

    assert!(!output.status.success(), "Check should fail for padding bytes violation");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("padding") && stderr.contains("exceeds budget"),
        "Should mention padding violation: {}",
        stderr
    );
}

// ============================================================================
// Error path tests
// ============================================================================

#[test]
fn test_inspect_nonexistent_file() {
    let output = std::process::Command::new("cargo")
        .args(["run", "--", "inspect", "/nonexistent/path/to/binary"])
        .output()
        .expect("Failed to run inspect command");

    assert!(!output.status.success(), "Should fail for nonexistent file");
}

#[test]
fn test_check_nonexistent_config() {
    let path = match get_fixture_path() {
        Some(p) => p,
        None => return,
    };

    let output = std::process::Command::new("cargo")
        .args([
            "run",
            "--",
            "check",
            path.to_str().unwrap(),
            "--config",
            "/nonexistent/config.yaml",
        ])
        .output()
        .expect("Failed to run check command");

    assert!(!output.status.success(), "Should fail for nonexistent config");
}

#[test]
fn test_check_invalid_yaml_config() {
    let path = match get_fixture_path() {
        Some(p) => p,
        None => return,
    };

    let config = create_temp_config("this is not valid yaml: [[[");

    let output = std::process::Command::new("cargo")
        .args(["run", "--", "check", path.to_str().unwrap(), "--config", config.to_str().unwrap()])
        .output()
        .expect("Failed to run check command");

    std::fs::remove_file(&config).ok();

    assert!(!output.status.success(), "Should fail for invalid YAML");
}

#[test]
fn test_check_nan_padding_percent() {
    let path = match get_fixture_path() {
        Some(p) => p,
        None => return,
    };

    // YAML parses .nan as NaN
    let config = create_temp_config(
        r#"
budgets:
  NoPadding:
    max_padding_percent: .nan
"#,
    );

    let output = std::process::Command::new("cargo")
        .args(["run", "--", "check", path.to_str().unwrap(), "--config", config.to_str().unwrap()])
        .output()
        .expect("Failed to run check command");

    std::fs::remove_file(&config).ok();

    assert!(!output.status.success(), "Should fail for NaN padding percent");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("finite") || stderr.contains("Invalid"),
        "Should reject NaN: {}",
        stderr
    );
}

#[test]
fn test_check_infinity_padding_percent() {
    let path = match get_fixture_path() {
        Some(p) => p,
        None => return,
    };

    // YAML parses .inf as infinity
    let config = create_temp_config(
        r#"
budgets:
  NoPadding:
    max_padding_percent: .inf
"#,
    );

    let output = std::process::Command::new("cargo")
        .args(["run", "--", "check", path.to_str().unwrap(), "--config", config.to_str().unwrap()])
        .output()
        .expect("Failed to run check command");

    std::fs::remove_file(&config).ok();

    assert!(!output.status.success(), "Should fail for infinity padding percent");
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("finite") || stderr.contains("Invalid") || stderr.contains("exceed 100"),
        "Should reject infinity: {}",
        stderr
    );
}

// ============================================================================
// C++ template tests
// ============================================================================

fn get_cpp_fixture_path() -> Option<std::path::PathBuf> {
    let dsym_path = std::path::Path::new(
        "tests/fixtures/bin/test_cpp_templates.dSYM/Contents/Resources/DWARF/test_cpp_templates",
    );
    if dsym_path.exists() {
        return Some(dsym_path.to_path_buf());
    }

    let exe_path = std::path::Path::new("tests/fixtures/bin/test_cpp_templates.exe");
    if exe_path.exists() {
        return Some(exe_path.to_path_buf());
    }

    let direct_path = std::path::Path::new("tests/fixtures/bin/test_cpp_templates");
    if direct_path.exists() {
        return Some(direct_path.to_path_buf());
    }

    None
}

#[test]
fn test_cpp_simple_template() {
    let path = match get_cpp_fixture_path() {
        Some(p) => p,
        None => return, // Skip if not compiled
    };

    let output = std::process::Command::new("cargo")
        .args([
            "run",
            "--",
            "inspect",
            path.to_str().unwrap(),
            "--filter",
            "Container<int>",
            "-o",
            "json",
        ])
        .output()
        .expect("Failed to run inspect");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("Invalid JSON");
    let structs = parsed["structs"].as_array().unwrap();

    assert!(!structs.is_empty(), "Should find Container<int>");
    let container = &structs[0];
    assert_eq!(container["name"], "Container<int>");
    assert_eq!(container["size"], 12);

    // Check members have proper types (not "unknown")
    let members = container["members"].as_array().unwrap();
    let value_member = members.iter().find(|m| m["name"] == "value").unwrap();
    assert_eq!(value_member["type_name"], "int", "Template member should have resolved type");
}

#[test]
fn test_cpp_nested_templates() {
    let path = match get_cpp_fixture_path() {
        Some(p) => p,
        None => return,
    };

    let output = std::process::Command::new("cargo")
        .args([
            "run",
            "--",
            "inspect",
            path.to_str().unwrap(),
            "--filter",
            "MapEntry",
            "-o",
            "json",
        ])
        .output()
        .expect("Failed to run inspect");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("Invalid JSON");
    let structs = parsed["structs"].as_array().unwrap();

    // Should find MapEntry with nested Vector template
    let complex = structs.iter().find(|s| s["name"].as_str().unwrap().contains("Vector"));
    assert!(complex.is_some(), "Should find MapEntry with nested Vector template");
}

#[test]
fn test_cpp_template_padding_detection() {
    let path = match get_cpp_fixture_path() {
        Some(p) => p,
        None => return,
    };

    let output = std::process::Command::new("cargo")
        .args([
            "run",
            "--",
            "inspect",
            path.to_str().unwrap(),
            "--filter",
            "Triple<char, int, char>",
            "-o",
            "json",
        ])
        .output()
        .expect("Failed to run inspect");

    assert!(output.status.success());

    let stdout = String::from_utf8_lossy(&output.stdout);
    let parsed: serde_json::Value = serde_json::from_str(&stdout).expect("Invalid JSON");
    let structs = parsed["structs"].as_array().unwrap();

    assert!(!structs.is_empty(), "Should find Triple<char, int, char>");
    let triple = &structs[0];

    // This template has 50% padding due to alignment
    let padding_pct = triple["metrics"]["padding_percentage"].as_f64().unwrap();
    assert!(padding_pct > 40.0, "Triple<char, int, char> should have significant padding");
}
