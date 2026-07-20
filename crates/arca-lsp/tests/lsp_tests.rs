use arca_lsp::LspServer;

#[test]
fn test_lsp_completion_and_hover() {
    let mut lsp = LspServer::new();
    let uri = "file:///main.arca";
    let code = "fn main() { println(\"Hello\") }";

    lsp.open_document(uri.into(), code.into());

    let completions = lsp.get_completion(uri, 1, 5);
    assert!(!completions.is_empty());
    assert_eq!(completions[0].label, "Array<T>");

    let hover = lsp.get_hover(uri, 1, 5);
    assert!(hover.is_some());
}

#[test]
fn test_lsp_formatting_and_diagnostics() {
    let mut lsp = LspServer::new();
    let uri = "file:///main.arca";
    let code = "fn main() { let x = 10 }";

    lsp.open_document(uri.into(), code.into());

    let formatted = lsp.format_document(uri).unwrap();
    assert!(!formatted.is_empty());

    let diags = lsp.diagnose(uri);
    assert!(diags.is_empty());
}
