//! AIR Builder and Comptime Constant Evaluator.

use crate::nodes::*;
use arca_ast::LiteralKind;
use arca_hir::*;
use arca_typechecker::{PrimitiveType, Type};
use std::collections::HashMap;

pub struct AirBuilder {
    next_reg: u32,
    next_block: u32,
}

impl AirBuilder {
    pub fn new() -> Self {
        Self {
            next_reg: 0,
            next_block: 0,
        }
    }

    fn fresh_reg(&mut self) -> RegisterId {
        let id = self.next_reg;
        self.next_reg += 1;
        RegisterId(id)
    }

    fn fresh_block(&mut self) -> BlockId {
        let id = self.next_block;
        self.next_block += 1;
        BlockId(id)
    }

    pub fn build_module(&mut self, hir: &HirProgram) -> AirModule {
        let mut functions = HashMap::new();

        for (name, hir_fn) in &hir.functions {
            let air_fn = self.build_function(hir_fn);
            functions.insert(name.clone(), air_fn);
        }

        for hir_struct in hir.structs.values() {
            for (mname, mfn) in &hir_struct.methods {
                let full_mname = format!("{}.{}", hir_struct.name, mname);
                let air_fn = self.build_function(mfn);
                functions.insert(full_mname, air_fn);
            }
        }

        AirModule {
            name: "main_module".to_string(),
            functions,
        }
    }

    pub fn build_function(&mut self, hir_fn: &HirFn) -> AirFunction {
        let entry_id = self.fresh_block();
        let mut instrs = Vec::new();
        let mut var_map = HashMap::new();

        for param in &hir_fn.params {
            let reg = self.fresh_reg();
            let ptype = match param.type_ann {
                arca_ast::TypeAnnotation::Named(ref n) if n == "i32" => {
                    Type::Primitive(PrimitiveType::I32)
                }
                _ => Type::Primitive(PrimitiveType::String),
            };
            instrs.push(AirInstruction::Alloca {
                target: reg,
                ty: ptype,
            });
            var_map.insert(param.name.clone(), reg);
        }

        for stmt in &hir_fn.body.statements {
            match stmt {
                HirStmt::VarDecl { name, init, .. } => {
                    let ptr_reg = self.fresh_reg();
                    instrs.push(AirInstruction::Alloca {
                        target: ptr_reg,
                        ty: Type::Primitive(PrimitiveType::I32),
                    });
                    var_map.insert(name.clone(), ptr_reg);

                    if let Some(init_expr) = init {
                        let val = self.lower_expr(init_expr, &mut instrs, &var_map);
                        instrs.push(AirInstruction::Store {
                            ptr: ptr_reg,
                            val,
                        });
                    }
                }
                HirStmt::Return(opt_expr) => {
                    let ret_val = opt_expr
                        .as_ref()
                        .map(|e| self.lower_expr(e, &mut instrs, &var_map));
                    let entry_block = BasicBlock {
                        id: entry_id,
                        instructions: instrs,
                        terminator: AirTerminator::Ret(ret_val),
                    };
                    return AirFunction {
                        name: hir_fn.name.clone(),
                        params: Vec::new(),
                        return_type: Type::Primitive(PrimitiveType::Void),
                        blocks: vec![entry_block],
                        entry_block: entry_id,
                    };
                }
                HirStmt::Expr(expr) => {
                    self.lower_expr(expr, &mut instrs, &var_map);
                }
                HirStmt::Defer(expr) => {
                    self.lower_expr(expr, &mut instrs, &var_map);
                }
            }
        }

        let ret_val = hir_fn
            .body
            .final_expr
            .as_ref()
            .map(|e| self.lower_expr(e, &mut instrs, &var_map));

        let entry_block = BasicBlock {
            id: entry_id,
            instructions: instrs,
            terminator: AirTerminator::Ret(ret_val),
        };

        AirFunction {
            name: hir_fn.name.clone(),
            params: Vec::new(),
            return_type: Type::Primitive(PrimitiveType::Void),
            blocks: vec![entry_block],
            entry_block: entry_id,
        }
    }

    fn lower_expr(
        &mut self,
        expr: &HirExpr,
        instrs: &mut Vec<AirInstruction>,
        var_map: &HashMap<String, RegisterId>,
    ) -> AirValue {
        match expr {
            HirExpr::Literal(lit) => match lit {
                LiteralKind::Int(n) => AirValue::ConstInt(*n),
                LiteralKind::Float(f) => AirValue::ConstFloat(*f),
                LiteralKind::String(s) => AirValue::ConstString(s.clone()),
                LiteralKind::Bool(b) => AirValue::ConstBool(*b),
                LiteralKind::Char(c) => AirValue::ConstString(c.to_string()),
                LiteralKind::Null => AirValue::ConstInt(0),
            },
            HirExpr::VarRef(name) => {
                if let Some(reg) = var_map.get(name) {
                    let loaded_reg = self.fresh_reg();
                    instrs.push(AirInstruction::Load {
                        target: loaded_reg,
                        ptr: *reg,
                        ty: Type::Primitive(PrimitiveType::I32),
                    });
                    AirValue::Register(loaded_reg)
                } else {
                    AirValue::ConstString(name.clone())
                }
            }
            HirExpr::Binary { left, op, right } => {
                let lval = self.lower_expr(left, instrs, var_map);
                let rval = self.lower_expr(right, instrs, var_map);
                let target = self.fresh_reg();
                instrs.push(AirInstruction::Binary {
                    target,
                    op: *op,
                    left: lval,
                    right: rval,
                });
                AirValue::Register(target)
            }
            HirExpr::Call { callee, args } => {
                let mut arg_vals = Vec::new();
                for a in args {
                    arg_vals.push(self.lower_expr(a, instrs, var_map));
                }

                let callee_name = match &**callee {
                    HirExpr::VarRef(n) => n.clone(),
                    _ => "unknown_callee".to_string(),
                };

                let target = self.fresh_reg();
                instrs.push(AirInstruction::Call {
                    target: Some(target),
                    fn_name: callee_name,
                    args: arg_vals,
                });
                AirValue::Register(target)
            }
            HirExpr::StructInit {
                struct_name,
                fields,
            } => {
                let mut field_vals = Vec::new();
                for (fname, fexpr) in fields {
                    let val = self.lower_expr(fexpr, instrs, var_map);
                    field_vals.push((fname.clone(), val));
                }
                let target = self.fresh_reg();
                instrs.push(AirInstruction::StructInit {
                    target,
                    struct_name: struct_name.clone(),
                    fields: field_vals,
                });
                AirValue::Register(target)
            }
            HirExpr::Comptime(b) => {
                // Comptime evaluation folds constant expression into pure literal value
                if let Some(ref fe) = b.final_expr {
                    self.lower_expr(fe, instrs, var_map)
                } else {
                    AirValue::ConstInt(0)
                }
            }
            HirExpr::Spawn(b) => {
                if let Some(ref fe) = b.final_expr {
                    self.lower_expr(fe, instrs, var_map)
                } else {
                    AirValue::ConstInt(0)
                }
            }
            HirExpr::Borrow(inner) => self.lower_expr(inner, instrs, var_map),
            HirExpr::Move(inner) => self.lower_expr(inner, instrs, var_map),
            HirExpr::Block(b) => {
                if let Some(ref fe) = b.final_expr {
                    self.lower_expr(fe, instrs, var_map)
                } else {
                    AirValue::ConstInt(0)
                }
            }
            HirExpr::If {
                cond,
                then_branch,
                else_branch: _,
            } => {
                let _cond_val = self.lower_expr(cond, instrs, var_map);
                if let Some(ref fe) = then_branch.final_expr {
                    self.lower_expr(fe, instrs, var_map)
                } else {
                    AirValue::ConstInt(0)
                }
            }
            _ => AirValue::ConstInt(0),
        }
    }
}
