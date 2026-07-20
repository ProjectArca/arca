//! Static type checking engine for Arca.

pub mod checker;
pub mod env;
pub mod types;

pub use checker::TypeChecker;
pub use env::TypeEnv;
pub use types::{FnType, PrimitiveType, Type};
