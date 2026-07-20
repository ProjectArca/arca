use arca_fmt::ArcaFormatter;

#[test]
fn test_canonical_formatting() {
    let unformatted = "fn main() { let x = 10; }";
    let formatter = ArcaFormatter::new();
    let formatted = formatter.format_source(unformatted);

    assert!(formatted.contains("fn main() {"));
    assert!(formatted.contains("  let x = 10"));
    assert!(formatted.contains("}"));
}
