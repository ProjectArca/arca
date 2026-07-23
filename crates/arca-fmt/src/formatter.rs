//! Canonical code formatter engine (`arcafmt`) for Arca source code.

use arca_lexer::{Lexer, TokenKind};

pub struct ArcaFormatter {
    indent_size: usize,
}

impl ArcaFormatter {
    pub fn new() -> Self {
        Self { indent_size: 4 }
    }

    pub fn format_source<S: AsRef<str>>(&self, source: S) -> String {
        let mut lexer = Lexer::new(source.as_ref());
        let tokens = lexer.tokenize_all();

        let mut out = String::new();
        let mut indent_level = 0;
        let mut at_line_start = true;
        let mut prev_token_kind: Option<TokenKind> = None;

        for token in tokens {
            if token.kind == TokenKind::Eof {
                break;
            }

            match &token.kind {
                TokenKind::OpenBrace => {
                    if !out.ends_with('\n') && !at_line_start {
                        out.push('\n');
                    }
                    out.push_str(&" ".repeat(indent_level * self.indent_size));
                    out.push('{');
                    out.push('\n');
                    indent_level += 1;
                    at_line_start = true;
                }
                TokenKind::CloseBrace => {
                    if !at_line_start {
                        out.push('\n');
                    }
                    if indent_level > 0 {
                        indent_level -= 1;
                    }
                    out.push_str(&" ".repeat(indent_level * self.indent_size));
                    out.push('}');
                    out.push('\n');
                    at_line_start = true;
                }
                TokenKind::Semicolon => {
                    out.push('\n');
                    at_line_start = true;
                }
                _ => {
                    if at_line_start {
                        out.push_str(&" ".repeat(indent_level * self.indent_size));
                        at_line_start = false;
                    } else if self.should_insert_space(prev_token_kind.as_ref(), &token.kind) {
                        out.push(' ');
                    }

                    out.push_str(&self.token_text(&token.kind));
                }
            }

            prev_token_kind = Some(token.kind.clone());
        }

        if !out.ends_with('\n') {
            out.push('\n');
        }

        out
    }

    fn token_text(&self, kind: &TokenKind) -> String {
        match kind {
            TokenKind::Identifier(id) => id.clone(),
            TokenKind::IntLiteral(n) => n.to_string(),
            TokenKind::FloatLiteral(f) => f.to_string(),
            TokenKind::StringLiteral(s) => format!("\"{}\"", s),
            TokenKind::CharLiteral(c) => format!("'{}'", c),
            _ => format!("{}", kind),
        }
    }

    fn should_insert_space(&self, prev: Option<&TokenKind>, curr: &TokenKind) -> bool {
        let Some(prev_kind) = prev else {
            return false;
        };

        match curr {
            TokenKind::Comma | TokenKind::CloseParen | TokenKind::CloseBracket | TokenKind::Dot => false,
            TokenKind::OpenParen => match prev_kind {
                TokenKind::Identifier(_) => false,
                _ => true,
            },
            _ => match prev_kind {
                TokenKind::OpenParen | TokenKind::OpenBracket | TokenKind::Dot => false,
                TokenKind::Comma => true,
                TokenKind::Assign
                | TokenKind::Plus
                | TokenKind::Minus
                | TokenKind::Star
                | TokenKind::Slash
                | TokenKind::Equal
                | TokenKind::NotEqual
                | TokenKind::Arrow
                | TokenKind::FatArrow => true,
                _ => true,
            },
        }
    }
}
