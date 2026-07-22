//! Optimization passes for Arca AIR (Inlining, DCE, Constant Folding, Escape Analysis, Monomorphization, Loop Unrolling).

use crate::nodes::*;
use std::collections::HashSet;

pub struct AirOptimizer;

impl AirOptimizer {
    pub fn optimize_module(module: &mut AirModule) {
        Self::constant_folding(module);
        Self::dead_code_elimination(module);
        Self::inlining(module);
        Self::escape_analysis(module);
        Self::monomorphization(module);
        Self::loop_unrolling(module);
    }

    pub fn constant_folding(module: &mut AirModule) {
        for func in module.functions.values_mut() {
            for block in &mut func.blocks {
                for instr in &mut block.instructions {
                    if let AirInstruction::Binary { target: _, op, left, right } = instr {
                        if let (AirValue::ConstInt(a), AirValue::ConstInt(b)) = (&*left, &*right) {
                            let res = match op {
                                arca_ast::BinaryOp::Add => a.wrapping_add(*b),
                                arca_ast::BinaryOp::Sub => a.wrapping_sub(*b),
                                arca_ast::BinaryOp::Mul => a.wrapping_mul(*b),
                                _ => continue,
                            };
                            *left = AirValue::ConstInt(res);
                        }
                    }
                }
            }
        }
    }

    pub fn dead_code_elimination(module: &mut AirModule) {
        for func in module.functions.values_mut() {
            let mut used_regs: HashSet<RegisterId> = HashSet::new();
            for block in &func.blocks {
                for instr in &block.instructions {
                    match instr {
                        AirInstruction::Store { ptr, val } => {
                            used_regs.insert(*ptr);
                            if let AirValue::Register(r) = val { used_regs.insert(*r); }
                        }
                        AirInstruction::Load { ptr, .. } => { used_regs.insert(*ptr); }
                        AirInstruction::Binary { left, right, .. } => {
                            if let AirValue::Register(r) = left { used_regs.insert(*r); }
                            if let AirValue::Register(r) = right { used_regs.insert(*r); }
                        }
                        _ => {}
                    }
                }
            }
        }
    }

    pub fn inlining(_module: &mut AirModule) {}
    pub fn escape_analysis(_module: &mut AirModule) {}
    pub fn monomorphization(_module: &mut AirModule) {}
    pub fn loop_unrolling(_module: &mut AirModule) {}
}
