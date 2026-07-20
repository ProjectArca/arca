//! Module dependency graph resolver and symbol import exporter for Arca.

use arca_ast::{Decl, Program};
use arca_diagnostics::Diagnostic;
use arca_lexer::Lexer;
use arca_parser::Parser;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ModuleNode {
    pub file_path: PathBuf,
    pub program: Program,
    pub exported_symbols: HashSet<String>,
    pub imports: Vec<(Vec<String>, String)>,
}

#[derive(Debug, Default)]
pub struct ModuleResolver {
    pub modules: HashMap<PathBuf, ModuleNode>,
    pub diagnostics: Vec<Diagnostic>,
}

impl ModuleResolver {
    pub fn new() -> Self {
        Self {
            modules: HashMap::new(),
            diagnostics: Vec::new(),
        }
    }

    pub fn resolve_package<P: AsRef<Path>>(&mut self, root_file: P) -> Result<(), Vec<Diagnostic>> {
        let canonical_root = root_file
            .as_ref()
            .canonicalize()
            .unwrap_or_else(|_| root_file.as_ref().to_path_buf());

        let mut visited = HashSet::new();
        let mut stack = Vec::new();

        self.load_module_recursive(&canonical_root, &mut visited, &mut stack);

        if self
            .diagnostics
            .iter()
            .any(|d| d.severity == arca_diagnostics::Severity::Error)
        {
            Err(std::mem::take(&mut self.diagnostics))
        } else {
            Ok(())
        }
    }

    fn load_module_recursive(
        &mut self,
        file_path: &Path,
        visited: &mut HashSet<PathBuf>,
        stack: &mut Vec<PathBuf>,
    ) {
        if stack.contains(&file_path.to_path_buf()) {
            let cycle_str = stack
                .iter()
                .map(|p| p.display().to_string())
                .collect::<Vec<_>>()
                .join(" -> ");
            self.diagnostics.push(Diagnostic::error(format!(
                "Cyclic module import detected: {} -> {}",
                cycle_str,
                file_path.display()
            )));
            return;
        }

        if visited.contains(file_path) {
            return;
        }

        visited.insert(file_path.to_path_buf());
        stack.push(file_path.to_path_buf());

        let source = match fs::read_to_string(file_path) {
            Ok(s) => s,
            Err(err) => {
                self.diagnostics.push(Diagnostic::error(format!(
                    "Failed to read module file '{}': {}",
                    file_path.display(),
                    err
                )));
                stack.pop();
                return;
            }
        };

        let lexer = Lexer::new(&source);
        let mut parser = Parser::new(lexer).with_file(file_path.display().to_string());
        let program = parser.parse_program();

        if !parser.diagnostics().is_empty() {
            self.diagnostics.extend(parser.diagnostics().to_vec());
        }

        let mut exported_symbols = HashSet::new();
        let mut imports = Vec::new();

        for decl in &program.declarations {
            match decl {
                Decl::Export { decl: inner, .. } => match &**inner {
                    Decl::Struct { name, .. }
                    | Decl::Enum { name, .. }
                    | Decl::Capability { name, .. } => {
                        exported_symbols.insert(name.clone());
                    }
                    Decl::Fn(f) => {
                        exported_symbols.insert(f.name.clone());
                    }
                    _ => {}
                },
                Decl::Import { items, source, .. } => {
                    imports.push((items.clone(), source.clone()));
                }
                _ => {}
            }
        }

        let parent_dir = file_path.parent().unwrap_or_else(|| Path::new("."));

        for (_, import_src) in &imports {
            if import_src.starts_with('.') {
                let resolved_path = self.resolve_relative_path(parent_dir, import_src);
                self.load_module_recursive(&resolved_path, visited, stack);
            }
        }

        self.modules.insert(
            file_path.to_path_buf(),
            ModuleNode {
                file_path: file_path.to_path_buf(),
                program,
                exported_symbols,
                imports,
            },
        );

        stack.pop();
    }

    fn resolve_relative_path(&self, base_dir: &Path, import_src: &str) -> PathBuf {
        let mut target = base_dir.join(import_src);
        if target.extension().is_none() {
            target.set_extension("arca");
        }
        target
            .canonicalize()
            .unwrap_or(target)
    }
}
