//! Package management and dependency resolution library for Arca.

pub mod lockfile;
pub mod manager;

pub use lockfile::{LockPackage, Lockfile};
pub use manager::PackageManager;
