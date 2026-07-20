//! Arca Language Server Protocol Engine (`arca-lsp`).

use arca_diagnostics::Diagnostic;
use arca_fmt::ArcaFormatter;
use arca_hir::Lowerer;
use arca_lexer::Lexer;
use arca_parser::Parser;
use arca_typechecker::TypeChecker;
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct CompletionItem {
    pub label: String,
    pub detail: String,
    pub kind: &'static str,
}

#[derive(Debug, Clone)]
pub struct HoverInfo {
    pub contents: String,
}

pub struct LspServer {
    documents: HashMap<String, String>,
}

impl LspServer {
    pub fn new() -> Self {
        Self {
            documents: HashMap::new(),
        }
    }

    pub fn open_document(&mut self, uri: String, text: String) {
        self.documents.insert(uri, text);
    }

    pub fn get_completion(&self, _uri: &str, _line: usize, _character: usize) -> Vec<CompletionItem> {
        vec![
            CompletionItem {
                label: "Array<T>".into(),
                detail: "Generic container array".into(),
                kind: "Class",
            },
            CompletionItem {
                label: "Map<K, V>".into(),
                detail: "Hash map dictionary".into(),
                kind: "Class",
            },
            CompletionItem {
                label: "println".into(),
                detail: "fn println(val: any)".into(),
                kind: "Function",
            },
            CompletionItem {
                label: "try".into(),
                detail: "Error propagation block".into(),
                kind: "Keyword",
            },
        ]
    }

    pub fn get_hover(&self, _uri: &str, _line: usize, _character: usize) -> Option<HoverInfo> {
        Some(HoverInfo {
            contents: "Arca Standard Built-in Primitive / Signature".into(),
        })
    }

    pub fn format_document(&self, uri: &str) -> Result<String, String> {
        if let Some(source) = self.documents.get(uri) {
            let formatter = ArcaFormatter::new();
            Ok(formatter.format_source(source.clone()))
        } else {
            Err("Document not found".into())
        }
    }

    pub fn diagnose(&self, uri: &str) -> Vec<Diagnostic> {
        if let Some(source) = self.documents.get(uri) {
            let lexer = Lexer::new(source);
            let mut parser = Parser::new(lexer).with_file(uri);
            let ast = parser.parse_program();

            if !parser.diagnostics().is_empty() {
                return parser.diagnostics().to_vec();
            }

            let lowerer = Lowerer::new();
            let hir = lowerer.lower_program(&ast);
            let mut checker = TypeChecker::new();
            checker.check_program(&hir)
        } else {
            Vec::new()
        }
    }
}
