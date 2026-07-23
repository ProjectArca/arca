//! Diagnostic and error reporting system for the Arca compiler.

use arca_ast::Span;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
    Note,
}

impl Severity {
    pub fn to_ansi(&self) -> &'static str {
        match self {
            Severity::Error => "\x1b[31m",
            Severity::Warning => "\x1b[33m",
            Severity::Note => "\x1b[36m",
        }
    }
    pub fn as_str(&self) -> &'static str {
        match self {
            Severity::Error => "error",
            Severity::Warning => "warning",
            Severity::Note => "note",
        }
    }
}

impl std::fmt::Display for Severity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    pub severity: Severity,
    pub message: String,
    pub span: Option<Span>,
    pub file_path: Option<String>,
    pub labels: Vec<(Option<Span>, String)>,
}

impl Diagnostic {
    pub fn error<S: Into<String>>(message: S) -> Self {
        Self {
            severity: Severity::Error,
            message: message.into(),
            span: None,
            file_path: None,
            labels: Vec::new(),
        }
    }

    pub fn warning<S: Into<String>>(message: S) -> Self {
        Self {
            severity: Severity::Warning,
            message: message.into(),
            span: None,
            file_path: None,
            labels: Vec::new(),
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

    pub fn with_label(mut self, span: Span, label: impl Into<String>) -> Self {
        self.labels.push((Some(span), label.into()));
        self
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.labels.push((None, note.into()));
        self
    }

    fn render_span_to_string(span: &Span, source: Option<&str>, is_primary: bool, use_color: bool) -> String {
        let mut out = String::new();
        if let Some(src) = source {
            let lines: Vec<&str> = src.lines().collect();
            let start_line = span.start_loc.line;
            let start_col = span.start_loc.column;
            let end_line = span.end_loc.line;
            let end_col = span.end_loc.column;

            let mut line = start_line;
            while line <= end_line && line <= lines.len() {
                let line_str = lines[line - 1];
                let col_start = if line == start_line { start_col } else { 1 };
                let col_end = if line == end_line { end_col } else { line_str.len() + 1 };

                let marker_start = col_start.saturating_sub(1);
                let marker_len = (col_end - col_start).max(1);

                if use_color {
                    out.push_str(&format!("  {} | {}\n", line, line_str));
                    out.push_str(&format!("  {} | {}{}{}\n", line, " ".repeat(marker_start), "\x1b[31m", "^".repeat(marker_len)));
                } else {
                    out.push_str(&format!("  {} | {}\n", line, line_str));
                    out.push_str(&format!("  {} | {}{}\n", line, " ".repeat(marker_start), "^".repeat(marker_len)));
                }
                line += 1;
            }
        }
        out
    }

    pub fn render(&self, source: Option<&str>) -> String {
        self.render_color(source)
    }

    pub fn render_color(&self, source: Option<&str>) -> String {
        let use_color = true;
        self.render_impl(source, use_color)
    }

    pub fn render_plain(&self, source: Option<&str>) -> String {
        let use_color = false;
        self.render_impl(source, use_color)
    }

    fn render_impl(&self, source: Option<&str>, use_color: bool) -> String {
        let file = self.file_path.as_deref().unwrap_or("<unknown>");
        let mut out = String::new();
        let reset = "\x1b[0m";

        if let Some(span) = self.span {
            let header = if use_color {
                format!("{}{}: {}{}", self.severity.to_ansi(), self.severity, reset, self.message)
            } else {
                format!("{}: {}", self.severity, self.message)
            };
            out.push_str(&format!("{}\n  --> {}:{}:{}\n", header, file, span.start_loc.line, span.start_loc.column));
            out.push_str(&Self::render_span_to_string(&span, source, true, use_color));
        } else {
            let header = if use_color {
                format!("{}{}: {}{}", self.severity.to_ansi(), self.severity, reset, self.message)
            } else {
                format!("{}: {}", self.severity, self.message)
            };
            out.push_str(&format!("{}\n", header));
        }

        for (span, label) in &self.labels {
            if let Some(span) = span {
                let label_line = if use_color {
                    format!("  {}{}: {}{}", self.severity.to_ansi(), self.severity, reset, label)
                } else {
                    format!("  : {}", label)
                };
                out.push_str(&label_line);
                out.push('\n');
                out.push_str(&Self::render_span_to_string(span, source, false, use_color));
            } else {
                let note = if use_color {
                    format!("  {}note: {}{}", "\x1b[36m", label, reset)
                } else {
                    format!("  note: {}", label)
                };
                out.push_str(&note);
                out.push('\n');
            }
        }

        out
    }
}
