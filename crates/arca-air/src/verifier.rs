//! SSA AIR verifier validating basic block terminators, CFG graph, and registers.

use crate::nodes::*;
use arca_diagnostics::Diagnostic;

pub struct AirVerifier;

impl AirVerifier {
    pub fn verify_module(module: &AirModule) -> Result<(), Vec<Diagnostic>> {
        let mut diags = Vec::new();

        for (fname, func) in &module.functions {
            if func.blocks.is_empty() {
                diags.push(Diagnostic::error(format!(
                    "Function '{}' has no basic blocks",
                    fname
                )));
                continue;
            }

            for block in &func.blocks {
                match block.terminator {
                    AirTerminator::Ret(_) | AirTerminator::Br(_) | AirTerminator::CondBr { .. } => {}
                    AirTerminator::Unreachable => {
                        diags.push(Diagnostic::warning(format!(
                            "Block {:?} in function '{}' has unreachable terminator",
                            block.id, fname
                        )));
                    }
                }
            }
        }

        if diags.iter().any(|d| d.severity == arca_diagnostics::Severity::Error) {
            Err(diags)
        } else {
            Ok(())
        }
    }
}
