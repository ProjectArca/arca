//! Token definitions for the Arca lexer.

use arca_ast::Span;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Keywords
    Let,
    Const,
    Fn,
    Struct,
    Enum,
    Impl,
    Extend,
    Capability,
    Import,
    Export,
    Borrow,
    Move,
    Unsafe,
    Comptime,
    Spawn,
    Defer,
    Match,
    If,
    Else,
    Return,
    For,
    While,
    Loop,
    Mut,
    Type,
    Interface,
    Async,
    Await,
    Actor,
    ErrorKw,
    Extern,
    True,
    False,
    Nil,
    Try,
    Group,

    // Identifiers & Literals
    Identifier(String),
    IntLiteral(i64),
    FloatLiteral(f64),
    StringLiteral(String),
    CharLiteral(char),

    // Operators & Punctuation
    Plus,          // +
    Minus,         // -
    Star,          // *
    Slash,         // /
    Percent,       // %
    Assign,        // =
    Equal,         // ==
    NotEqual,      // !=
    Less,          // <
    LessEqual,     // <=
    Greater,       // >
    GreaterEqual,  // >=
    And,           // &&
    Or,           // ||
    Pipe,         // |
    Not,           // !
    Question,      // ?
    OptionalChain, // ?.
    NullCoalesce,  // ??
    Arrow,         // ->
    FatArrow,      // =>
    ColonColon,    // ::
    Colon,         // :
    Semicolon,     // ;
    Comma,         // ,
    Dot,           // .

    // Delimiters
    OpenParen,     // (
    CloseParen,    // )
    OpenBrace,     // {
    CloseBrace,    // }
    OpenBracket,   // [
    CloseBracket,  // ]

    // Special
    Eof,
    Error(String),
}

impl fmt::Display for TokenKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenKind::Let => write!(f, "let"),
            TokenKind::Const => write!(f, "const"),
            TokenKind::Fn => write!(f, "fn"),
            TokenKind::Struct => write!(f, "struct"),
            TokenKind::Enum => write!(f, "enum"),
            TokenKind::Impl => write!(f, "impl"),
            TokenKind::Extend => write!(f, "extend"),
            TokenKind::Capability => write!(f, "capability"),
            TokenKind::Import => write!(f, "import"),
            TokenKind::Export => write!(f, "export"),
            TokenKind::Borrow => write!(f, "borrow"),
            TokenKind::Move => write!(f, "move"),
            TokenKind::Unsafe => write!(f, "unsafe"),
            TokenKind::Comptime => write!(f, "comptime"),
            TokenKind::Spawn => write!(f, "spawn"),
            TokenKind::Defer => write!(f, "defer"),
            TokenKind::Match => write!(f, "match"),
            TokenKind::If => write!(f, "if"),
            TokenKind::Else => write!(f, "else"),
            TokenKind::Return => write!(f, "return"),
            TokenKind::For => write!(f, "for"),
            TokenKind::While => write!(f, "while"),
            TokenKind::Loop => write!(f, "loop"),
            TokenKind::Mut => write!(f, "mut"),
            TokenKind::Type => write!(f, "type"),
            TokenKind::Interface => write!(f, "interface"),
            TokenKind::Async => write!(f, "async"),
            TokenKind::Await => write!(f, "await"),
            TokenKind::Actor => write!(f, "actor"),
            TokenKind::ErrorKw => write!(f, "error"),
            TokenKind::Extern => write!(f, "extern"),
            TokenKind::True => write!(f, "true"),
            TokenKind::False => write!(f, "false"),
            TokenKind::Nil => write!(f, "nil"),
            TokenKind::Try => write!(f, "try"),
            TokenKind::Group => write!(f, "group"),
            TokenKind::Identifier(id) => write!(f, "Identifier({})", id),
            TokenKind::IntLiteral(n) => write!(f, "IntLiteral({})", n),
            TokenKind::FloatLiteral(fl) => write!(f, "FloatLiteral({})", fl),
            TokenKind::StringLiteral(s) => write!(f, "StringLiteral(\"{}\")", s),
            TokenKind::CharLiteral(c) => write!(f, "CharLiteral('{}')", c),
            TokenKind::Plus => write!(f, "+"),
            TokenKind::Minus => write!(f, "-"),
            TokenKind::Star => write!(f, "*"),
            TokenKind::Slash => write!(f, "/"),
            TokenKind::Percent => write!(f, "%"),
            TokenKind::Assign => write!(f, "="),
            TokenKind::Equal => write!(f, "=="),
            TokenKind::NotEqual => write!(f, "!="),
            TokenKind::Less => write!(f, "<"),
            TokenKind::LessEqual => write!(f, "<="),
            TokenKind::Greater => write!(f, ">"),
            TokenKind::GreaterEqual => write!(f, ">="),
            TokenKind::And => write!(f, "&&"),
            TokenKind::Or => write!(f, "||"),
            TokenKind::Pipe => write!(f, "|"),
            TokenKind::Not => write!(f, "!"),
            TokenKind::Question => write!(f, "?"),
            TokenKind::OptionalChain => write!(f, "?."),
            TokenKind::NullCoalesce => write!(f, "??"),
            TokenKind::Arrow => write!(f, "->"),
            TokenKind::FatArrow => write!(f, "=>"),
            TokenKind::ColonColon => write!(f, "::"),
            TokenKind::Colon => write!(f, ":"),
            TokenKind::Semicolon => write!(f, ";"),
            TokenKind::Comma => write!(f, ","),
            TokenKind::Dot => write!(f, "."),
            TokenKind::OpenParen => write!(f, "("),
            TokenKind::CloseParen => write!(f, ")"),
            TokenKind::OpenBrace => write!(f, "{{"),
            TokenKind::CloseBrace => write!(f, "}}"),
            TokenKind::OpenBracket => write!(f, "["),
            TokenKind::CloseBracket => write!(f, "]"),
            TokenKind::Eof => write!(f, "EOF"),
            TokenKind::Error(e) => write!(f, "Error({})", e),
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    pub fn new(kind: TokenKind, span: Span) -> Self {
        Self { kind, span }
    }
}
