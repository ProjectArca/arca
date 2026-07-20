//! Lexer library for the Arca programming language.

pub mod lexer;
pub mod token;

pub use lexer::Lexer;
pub use token::{Token, TokenKind};
