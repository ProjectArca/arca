//! Capability definitions, impl blocks, and vtable metadata.

use arca_typechecker::FnType;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CapabilityDef {
    pub name: String,
    pub methods: HashMap<String, FnType>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ImplBlock {
    pub capability_name: String,
    pub target_type: String,
    pub methods: HashMap<String, FnType>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct MethodSlot {
    pub method_name: String,
    pub index: usize,
    pub fn_type: FnType,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CapabilityVTable {
    pub capability_name: String,
    pub target_type: String,
    pub slots: Vec<MethodSlot>,
}
