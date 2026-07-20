//! Capability resolution, impl conformance checker, and dynamic dispatch vtable generator for Arca.

pub mod checker;
pub mod model;

pub use checker::CapabilityRegistry;
pub use model::{CapabilityDef, CapabilityVTable, ImplBlock, MethodSlot};
