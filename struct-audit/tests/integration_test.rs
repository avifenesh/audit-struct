use struct_audit::{analyze_layout, BinaryData, DwarfContext};

#[test]
#[ignore] // Requires test fixture to be compiled
fn test_parse_simple_struct() {
    // This test requires the test fixture to be compiled first:
    // gcc -g -o tests/fixtures/bin/test_simple tests/fixtures/test_simple.c
    let path = std::path::Path::new("tests/fixtures/bin/test_simple");
    if !path.exists() {
        eprintln!("Test fixture not compiled. Run: gcc -g -o tests/fixtures/bin/test_simple tests/fixtures/test_simple.c");
        return;
    }

    let binary = BinaryData::load(path).expect("Failed to load binary");
    let loaded = binary.load_dwarf().expect("Failed to load DWARF");
    let dwarf = DwarfContext::new(&loaded);

    let mut layouts = dwarf
        .find_structs(Some("NoPadding"))
        .expect("Failed to parse structs");

    assert_eq!(layouts.len(), 1);
    let layout = &mut layouts[0];

    analyze_layout(layout, 64);

    assert_eq!(layout.name, "NoPadding");
    assert_eq!(layout.size, 12);
    assert_eq!(layout.members.len(), 3);
    assert_eq!(layout.metrics.padding_bytes, 0);
}

#[test]
#[ignore] // Requires test fixture to be compiled
fn test_detect_padding() {
    let path = std::path::Path::new("tests/fixtures/bin/test_simple");
    if !path.exists() {
        return;
    }

    let binary = BinaryData::load(path).expect("Failed to load binary");
    let loaded = binary.load_dwarf().expect("Failed to load DWARF");
    let dwarf = DwarfContext::new(&loaded);

    let mut layouts = dwarf
        .find_structs(Some("InternalPadding"))
        .expect("Failed to parse structs");

    assert_eq!(layouts.len(), 1);
    let layout = &mut layouts[0];

    analyze_layout(layout, 64);

    assert_eq!(layout.name, "InternalPadding");
    assert_eq!(layout.size, 16);
    assert!(layout.metrics.padding_bytes > 0);
    assert!(!layout.metrics.padding_holes.is_empty());
}
