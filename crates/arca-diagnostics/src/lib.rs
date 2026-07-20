//! Diagnostic and error reporting system for the Arca compiler.

use arca_ast::Span;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Note,
}

impl fmt::Display for Severity {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Severity::Error => write!(f, "error"),
            Severity::Warning => write!(f, "warning"),
            Severity::Note => write!(f, "note"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub severity: Severity,
    pub message: String,
    pub span: Option<Span>,
    pub file_path: Option<String>,
}

impl Diagnostic {
    pub fn error<S: Into<String>>(message: S) -> Self {
        Self {
            severity: Severity::Error,
            message: message.into(),
            span: None,
            file_path: None,
        }
    }

    pub fn warning<S: Into<String>>(message: S) -> Self {
        Self {
            severity: Severity::Warning,
            message: message.into(),
            span: None,
            file_path: None,
        }
    }

    pub fn with_span(mut self, span: Span) -> Self {
        self.span = Some(span);
        self
    }

    pub fn with_file<S: Into<String>>(mut self, file_path: S) -> Self {
        self.file_path = Some(file_path.into());
        self
    }

    pub fn render(&self, source: Option<&str>) -> String {
        let file = self.file_path.as_deref().unwrap_or("<unknown>");
        let mut out = String::new();

        if let Some(span) = self.span {
            out.push_str(&format!(
                "{}: {}: {}\n  --> {}:{}:{}\n",
                self.severity,
                self.message,
                file,
                span.start_loc.line,
                span.start_loc.column,
                file
            ));

            if let Some(src) = source {
                let lines: Vec<&str> = src.lines().collect();
                if span.start_loc.line > 0 && span.start_loc.line <= lines.len() {
                    let line_str = lines[span.start_loc.line - 1];
                    let line_num = span.start_loc.line;
                    let col = span.start_loc.column;

                    out.push_str(&format!("{:>4} | {}\n", line_num, line_str));
                    out.push_str(&format!(
                        "     | {}{}\n",
                        " ".repeat(col.saturating_sub(1)),
                        "^"
                    ));
                }
            }
        } else {
            out.push_str(&format!("{}: {}\n", self.severity, self.message));
        }

        out
    }
}
