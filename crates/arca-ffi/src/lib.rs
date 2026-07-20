//! Zero-cost C ABI Native Interoperability library for Arca.

pub mod abi;
pub mod resolver;

pub use abi::{CallingConvention, CPrimitive, CStructField, CStructLayout};
pub use resolver::{ExternFn, FfiResolver};
