//! High-performance UTF-8 lexer for Arca.

use crate::token::{Token, TokenKind};
use arca_ast::{Location, Span};
use std::iter::Peekable;
use std::str::CharIndices;

pub struct Lexer<'a> {
    source: &'a str,
    chars: Peekable<CharIndices<'a>>,
    current_byte: usize,
    line: usize,
    column: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(source: &'a str) -> Self {
        // Strip UTF-8 BOM if present
        let source_clean = if source.starts_with('\u{FEFF}') {
            &source[3..]
        } else {
            source
        };

        Self {
            source: source_clean,
            chars: source_clean.char_indices().peekable(),
            current_byte: 0,
            line: 1,
            column: 1,
        }
    }

    pub fn source(&self) -> &'a str {
        self.source
    }

    fn peek_char(&mut self) -> Option<char> {
        self.chars.peek().map(|(_, c)| *c)
    }

    fn next_char(&mut self) -> Option<char> {
        if let Some((idx, c)) = self.chars.next() {
            self.current_byte = idx + c.len_utf8();
            if c == '\n' {
                self.line += 1;
                self.column = 1;
            } else {
                self.column += 1;
            }
            Some(c)
        } else {
            None
        }
    }

    fn current_location(&self) -> Location {
        Location {
            line: self.line,
            column: self.column,
        }
    }

    pub fn tokenize_all(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();
        loop {
            let token = self.next_token();
            let is_eof = token.kind == TokenKind::Eof;
            tokens.push(token);
            if is_eof {
                break;
            }
        }
        tokens
    }

    pub fn next_token(&mut self) -> Token {
        self.skip_whitespace_and_comments();

        let start_byte = self.current_byte;
        let start_loc = self.current_location();

        let ch = match self.next_char() {
            Some(c) => c,
            None => {
                return Token::new(
                    TokenKind::Eof,
                    Span::new(start_byte, start_byte, start_loc, start_loc),
                );
            }
        };

        let kind = match ch {
            '(' => TokenKind::OpenParen,
            ')' => TokenKind::CloseParen,
            '{' => TokenKind::OpenBrace,
            '}' => TokenKind::CloseBrace,
            '[' => TokenKind::OpenBracket,
            ']' => TokenKind::CloseBracket,
            ';' => TokenKind::Semicolon,
            ',' => TokenKind::Comma,
            '+' => TokenKind::Plus,
            '*' => TokenKind::Star,
            '%' => TokenKind::Percent,

            '-' => {
                if self.peek_char() == Some('>') {
                    self.next_char();
                    TokenKind::Arrow
                } else {
                    TokenKind::Minus
                }
            }

            '/' => TokenKind::Slash,

            '=' => {
                if self.peek_char() == Some('=') {
                    self.next_char();
                    TokenKind::Equal
                } else if self.peek_char() == Some('>') {
                    self.next_char();
                    TokenKind::FatArrow
                } else {
                    TokenKind::Assign
                }
            }

            '!' => {
                if self.peek_char() == Some('=') {
                    self.next_char();
                    TokenKind::NotEqual
                } else {
                    TokenKind::Not
                }
            }

            '<' => {
                if self.peek_char() == Some('=') {
                    self.next_char();
                    TokenKind::LessEqual
                } else {
                    TokenKind::Less
                }
            }

            '>' => {
                if self.peek_char() == Some('=') {
                    self.next_char();
                    TokenKind::GreaterEqual
                } else {
                    TokenKind::Greater
                }
            }

            '&' => {
                if self.peek_char() == Some('&') {
                    self.next_char();
                    TokenKind::And
                } else {
                    TokenKind::Error("Unexpected '&', did you mean '&&'?".into())
                }
            }

            '|' => {
                if self.peek_char() == Some('|') {
                    self.next_char();
                    TokenKind::Or
                } else {
                    TokenKind::Error("Unexpected '|', did you mean '||'?".into())
                }
            }

            '?' => {
                if self.peek_char() == Some('.') {
                    self.next_char();
                    TokenKind::OptionalChain
                } else if self.peek_char() == Some('?') {
                    self.next_char();
                    TokenKind::NullCoalesce
                } else {
                    TokenKind::Question
                }
            }

            ':' => {
                if self.peek_char() == Some(':') {
                    self.next_char();
                    TokenKind::ColonColon
                } else {
                    TokenKind::Colon
                }
            }

            '.' => TokenKind::Dot,

            '"' => self.scan_string(start_byte),
            '\'' => self.scan_char(),

            c if c.is_ascii_digit() => self.scan_number(c),
            c if is_ident_start(c) => self.scan_identifier_or_keyword(c),

            unsupported => TokenKind::Error(format!("Illegal character: '{}'", unsupported)),
        };

        let end_byte = self.current_byte;
        let end_loc = self.current_location();

        Token::new(kind, Span::new(start_byte, end_byte, start_loc, end_loc))
    }

    fn skip_whitespace_and_comments(&mut self) {
        while let Some(c) = self.peek_char() {
            if c.is_whitespace() {
                self.next_char();
            } else if c == '/' {
                // Peek ahead for comment
                let mut clone = self.chars.clone();
                clone.next(); // skip current '/'
                if let Some((_, next_c)) = clone.next() {
                    if next_c == '/' {
                        // Line comment
                        self.next_char(); // '/'
                        self.next_char(); // '/'
                        while let Some(ch) = self.peek_char() {
                            if ch == '\n' {
                                break;
                            }
                            self.next_char();
                        }
                    } else if next_c == '*' {
                        // Block comment
                        self.next_char(); // '/'
                        self.next_char(); // '*'
                        while let Some(ch) = self.next_char() {
                            if ch == '*' && self.peek_char() == Some('/') {
                                self.next_char();
                                break;
                            }
                        }
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            } else {
                break;
            }
        }
    }

    fn scan_identifier_or_keyword(&mut self, first: char) -> TokenKind {
        let mut text = String::new();
        text.push(first);

        while let Some(c) = self.peek_char() {
            if is_ident_continue(c) {
                text.push(c);
                self.next_char();
            } else {
                break;
            }
        }

        match text.as_str() {
            "let" => TokenKind::Let,
            "const" => TokenKind::Const,
            "fn" => TokenKind::Fn,
            "struct" => TokenKind::Struct,
            "enum" => TokenKind::Enum,
            "impl" => TokenKind::Impl,
            "extend" => TokenKind::Extend,
            "capability" => TokenKind::Capability,
            "import" => TokenKind::Import,
            "export" => TokenKind::Export,
            "borrow" => TokenKind::Borrow,
            "move" => TokenKind::Move,
            "unsafe" => TokenKind::Unsafe,
            "comptime" => TokenKind::Comptime,
            "spawn" => TokenKind::Spawn,
            "defer" => TokenKind::Defer,
            "match" => TokenKind::Match,
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "return" => TokenKind::Return,
            "for" => TokenKind::For,
            "while" => TokenKind::While,
            "loop" => TokenKind::Loop,
            "mut" => TokenKind::Mut,
            "type" => TokenKind::Type,
            "interface" => TokenKind::Interface,
            "async" => TokenKind::Async,
            "await" => TokenKind::Await,
            "actor" => TokenKind::Actor,
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            "nil" => TokenKind::Nil,
            _ => TokenKind::Identifier(text),
        }
    }

    fn scan_number(&mut self, first: char) -> TokenKind {
        let mut raw = String::new();
        raw.push(first);

        // Check for 0x (hex) or 0b (binary)
        if first == '0' {
            if let Some('x') | Some('X') = self.peek_char() {
                raw.push(self.next_char().unwrap());
                while let Some(c) = self.peek_char() {
                    if c.is_ascii_hexdigit() || c == '_' {
                        if c != '_' {
                            raw.push(c);
                        }
                        self.next_char();
                    } else {
                        break;
                    }
                }
                return match i64::from_str_radix(&raw[2..], 16) {
                    Ok(val) => TokenKind::IntLiteral(val),
                    Err(_) => TokenKind::Error("Invalid hexadecimal literal".into()),
                };
            } else if let Some('b') | Some('B') = self.peek_char() {
                raw.push(self.next_char().unwrap());
                while let Some(c) = self.peek_char() {
                    if c == '0' || c == '1' || c == '_' {
                        if c != '_' {
                            raw.push(c);
                        }
                        self.next_char();
                    } else {
                        break;
                    }
                }
                return match i64::from_str_radix(&raw[2..], 2) {
                    Ok(val) => TokenKind::IntLiteral(val),
                    Err(_) => TokenKind::Error("Invalid binary literal".into()),
                };
            }
        }

        let mut is_float = false;

        while let Some(c) = self.peek_char() {
            if c.is_ascii_digit() || c == '_' {
                if c != '_' {
                    raw.push(c);
                }
                self.next_char();
            } else if c == '.' && !is_float {
                // Peek next to ensure it's not range `..` or method call
                let mut clone = self.chars.clone();
                clone.next();
                if let Some((_, next_c)) = clone.next() {
                    if next_c.is_ascii_digit() {
                        is_float = true;
                        raw.push(c);
                        self.next_char(); // consume '.'
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            } else {
                break;
            }
        }

        if is_float {
            match raw.parse::<f64>() {
                Ok(val) => TokenKind::FloatLiteral(val),
                Err(_) => TokenKind::Error("Invalid floating point literal".into()),
            }
        } else {
            match raw.parse::<i64>() {
                Ok(val) => TokenKind::IntLiteral(val),
                Err(_) => TokenKind::Error("Invalid integer literal".into()),
            }
        }
    }

    fn scan_string(&mut self, _start_byte: usize) -> TokenKind {
        let mut s = String::new();
        while let Some(c) = self.next_char() {
            match c {
                '"' => return TokenKind::StringLiteral(s),
                '\\' => {
                    if let Some(escaped) = self.next_char() {
                        match escaped {
                            'n' => s.push('\n'),
                            't' => s.push('\t'),
                            'r' => s.push('\r'),
                            '\\' => s.push('\\'),
                            '"' => s.push('"'),
                            '0' => s.push('\0'),
                            other => s.push(other),
                        }
                    } else {
                        return TokenKind::Error("Unterminated escape sequence in string".into());
                    }
                }
                ch => s.push(ch),
            }
        }
        TokenKind::Error("Unterminated string literal".into())
    }

    fn scan_char(&mut self) -> TokenKind {
        let ch = match self.next_char() {
            Some('\'') => return TokenKind::Error("Empty character literal".into()),
            Some('\\') => match self.next_char() {
                Some('n') => '\n',
                Some('t') => '\t',
                Some('r') => '\r',
                Some('\\') => '\\',
                Some('\'') => '\'',
                Some(other) => other,
                None => return TokenKind::Error("Unterminated escape in character literal".into()),
            },
            Some(c) => c,
            None => return TokenKind::Error("Unterminated character literal".into()),
        };

        if self.next_char() == Some('\'') {
            TokenKind::CharLiteral(ch)
        } else {
            TokenKind::Error("Unterminated character literal".into())
        }
    }
}

fn is_ident_start(c: char) -> bool {
    c.is_alphabetic() || c == '_' || c == '$'
}

fn is_ident_continue(c: char) -> bool {
    c.is_alphanumeric() || c == '_' || c == '$'
}
