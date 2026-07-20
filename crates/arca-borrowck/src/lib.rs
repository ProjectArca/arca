//! Guided Ownership, Borrow Checker, and Memory Analyzer for Arca.

pub mod checker;
pub mod state;

pub use checker::BorrowChecker;
pub use state::{OwnershipTracker, VarState};
