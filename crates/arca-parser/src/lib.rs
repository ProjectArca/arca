//! Parser library for the Arca programming language.

pub mod parser;
pub mod precedence;

pub use parser::Parser;
pub use precedence::Precedence;
