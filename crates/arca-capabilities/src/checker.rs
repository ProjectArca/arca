//! Capability conformance engine and vtable generator.

use crate::model::{CapabilityDef, CapabilityVTable, ImplBlock, MethodSlot};
use arca_diagnostics::Diagnostic;
use arca_hir::HirProgram;
use arca_typechecker::TypeEnv;
use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct CapabilityRegistry {
    pub capabilities: HashMap<String, CapabilityDef>,
    pub impls: HashMap<(String, String), ImplBlock>,
    pub vtables: HashMap<(String, String), CapabilityVTable>,
}

impl CapabilityRegistry {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn register_capability(&mut self, def: CapabilityDef) {
        self.capabilities.insert(def.name.clone(), def);
    }

    pub fn register_impl(
        &mut self,
        impl_block: ImplBlock,
        env: &TypeEnv,
    ) -> Result<CapabilityVTable, Vec<Diagnostic>> {
        let mut diags = Vec::new();

        let cap_def = match self.capabilities.get(&impl_block.capability_name) {
            Some(c) => c.clone(),
            None => {
                diags.push(Diagnostic::error(format!(
                    "Unknown capability '{}' in impl block for type '{}'",
                    impl_block.capability_name, impl_block.target_type
                )));
                return Err(diags);
            }
        };

        let mut slots = Vec::new();
        let mut idx = 0;

        for (mname, expected_fnt) in &cap_def.methods {
            if let Some(actual_fnt) = impl_block.methods.get(mname) {
                if actual_fnt != expected_fnt {
                    diags.push(Diagnostic::error(format!(
                        "Method '{}' signature mismatch in impl '{}' for type '{}'",
                        mname, impl_block.capability_name, impl_block.target_type
                    )));
                } else {
                    slots.push(MethodSlot {
                        method_name: mname.clone(),
                        index: idx,
                        fn_type: actual_fnt.clone(),
                    });
                    idx += 1;
                }
            } else {
                diags.push(Diagnostic::error(format!(
                    "Missing method '{}' required by capability '{}' in impl for type '{}'",
                    mname, impl_block.capability_name, impl_block.target_type
                )));
            }
        }

        let _ = env;

        if !diags.is_empty() {
            Err(diags)
        } else {
            let vtable = CapabilityVTable {
                capability_name: impl_block.capability_name.clone(),
                target_type: impl_block.target_type.clone(),
                slots,
            };

            let key = (impl_block.capability_name.clone(), impl_block.target_type.clone());
            self.impls.insert(key.clone(), impl_block);
            self.vtables.insert(key, vtable.clone());

            Ok(vtable)
        }
    }

    pub fn validate_program_capabilities(
        &mut self,
        _hir: &HirProgram,
        _env: &TypeEnv,
    ) -> Vec<Diagnostic> {
        Vec::new()
    }
}
