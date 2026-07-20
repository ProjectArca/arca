//! Operator precedence levels for the Arca Pratt Parser.

use arca_lexer::TokenKind;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Precedence {
    Lowest = 0,
    Assign,       // =
    NullCoalesce, // ??
    LogicalOr,    // ||
    LogicalAnd,   // &&
    Equality,     // ==, !=
    Relational,   // <, <=, >, >=
    Additive,     // +, -
    Multiplicative, // *, /, %
    Unary,        // !, -
    CallMember,   // (), ., ?., []
    Primary,
}

impl Precedence {
    pub fn for_token(token_kind: &TokenKind) -> Precedence {
        match token_kind {
            TokenKind::Assign => Precedence::Assign,
            TokenKind::NullCoalesce => Precedence::NullCoalesce,
            TokenKind::Or => Precedence::LogicalOr,
            TokenKind::And => Precedence::LogicalAnd,

            TokenKind::Equal | TokenKind::NotEqual => Precedence::Equality,

            TokenKind::Less
            | TokenKind::LessEqual
            | TokenKind::Greater
            | TokenKind::GreaterEqual => Precedence::Relational,

            TokenKind::Plus | TokenKind::Minus => Precedence::Additive,

            TokenKind::Star | TokenKind::Slash | TokenKind::Percent => Precedence::Multiplicative,

            TokenKind::OpenParen
            | TokenKind::Dot
            | TokenKind::OptionalChain
            | TokenKind::OpenBracket => Precedence::CallMember,

            _ => Precedence::Lowest,
        }
    }
}
