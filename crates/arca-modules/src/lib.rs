//! Package manifest and multi-file module resolver library for Arca.

pub mod manifest;
pub mod resolver;

pub use manifest::{PackageManifest, PackageMetadata};
pub use resolver::{ModuleNode, ModuleResolver};
