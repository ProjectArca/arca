//! High-level Intermediate Representation (HIR) and Lowering Engine for Arca.

use arca_ast::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HirExpr {
    Literal(LiteralKind),
    VarRef(String),
    Binary {
        left: Box<HirExpr>,
        op: BinaryOp,
        right: Box<HirExpr>,
    },
    Unary {
        op: UnaryOp,
        expr: Box<HirExpr>,
    },
    Call {
        callee: Box<HirExpr>,
        args: Vec<HirExpr>,
    },
    Member {
        object: Box<HirExpr>,
        property: String,
        is_optional: bool,
    },
    StructInit {
        struct_name: String,
        fields: Vec<(String, HirExpr)>,
    },
    If {
        cond: Box<HirExpr>,
        then_branch: HirBlock,
        else_branch: Option<Box<HirExpr>>,
    },
    Match {
        value: Box<HirExpr>,
        arms: Vec<HirMatchArm>,
    },
    Block(HirBlock),
    Borrow(Box<HirExpr>),
    Move(Box<HirExpr>),
    Comptime(HirBlock),
    Spawn(HirBlock),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HirBlock {
    pub statements: Vec<HirStmt>,
    pub final_expr: Option<Box<HirExpr>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HirMatchArm {
    pub pattern: Pattern,
    pub guard: Option<HirExpr>,
    pub body: HirExpr,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum HirStmt {
    VarDecl {
        is_const: bool,
        name: String,
        type_ann: Option<TypeAnnotation>,
        init: Option<HirExpr>,
    },
    Return(Option<HirExpr>),
    Defer(HirExpr),
    Expr(HirExpr),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HirFn {
    pub name: String,
    pub params: Vec<ParamDef>,
    pub return_type: Option<TypeAnnotation>,
    pub throws_type: Option<TypeAnnotation>,
    pub body: HirBlock,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HirStruct {
    pub name: String,
    pub fields: Vec<FieldDef>,
    pub methods: HashMap<String, HirFn>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HirProgram {
    pub structs: HashMap<String, HirStruct>,
    pub functions: HashMap<String, HirFn>,
}

pub struct Lowerer {
    structs: HashMap<String, HirStruct>,
    functions: HashMap<String, HirFn>,
}

impl Lowerer {
    pub fn new() -> Self {
        Self {
            structs: HashMap::new(),
            functions: HashMap::new(),
        }
    }

    pub fn lower_program(mut self, program: &Program) -> HirProgram {
        // First pass: collect struct definitions
        for decl in &program.declarations {
            if let Decl::Struct { name, fields, methods, .. } = decl {
                let mut method_map = HashMap::new();
                for m in methods {
                    let hir_m = self.lower_fn(m);
                    method_map.insert(m.name.clone(), hir_m);
                }
                self.structs.insert(
                    name.clone(),
                    HirStruct {
                        name: name.clone(),
                        fields: fields.clone(),
                        methods: method_map,
                    },
                );
            }
        }

        // Second pass: lower extend blocks by merging into target structs
        let mut extensions: HashMap<String, Vec<HirFn>> = HashMap::new();
        for decl in &program.declarations {
            if let Decl::Extend { target_name, methods, .. } = decl {
                let mut hir_methods = Vec::new();
                for m in methods {
                    hir_methods.push(self.lower_fn(m));
                }
                extensions
                    .entry(target_name.clone())
                    .or_default()
                    .extend(hir_methods);
            }
        }

        for (target_name, hir_methods) in extensions {
            if let Some(target_struct) = self.structs.get_mut(&target_name) {
                for m in hir_methods {
                    target_struct.methods.insert(m.name.clone(), m);
                }
            }
        }

        // Third pass: lower top-level functions
        for decl in &program.declarations {
            if let Decl::Fn(fndecl) = decl {
                let hir_fn = self.lower_fn(fndecl);
                self.functions.insert(fndecl.name.clone(), hir_fn);
            }
        }

        HirProgram {
            structs: self.structs,
            functions: self.functions,
        }
    }

    fn lower_fn(&mut self, fndecl: &FnDecl) -> HirFn {
        HirFn {
            name: fndecl.name.clone(),
            params: fndecl.params.clone(),
            return_type: fndecl.return_type.clone(),
            throws_type: fndecl.throws_type.clone(),
            body: self.lower_block(&fndecl.body),
        }
    }

    fn lower_block(&mut self, block: &BlockExpr) -> HirBlock {
        let mut statements = Vec::new();
        for stmt in &block.statements {
            statements.push(self.lower_stmt(stmt));
        }

        let final_expr = block.final_expr.as_ref().map(|e| Box::new(self.lower_expr(e)));

        HirBlock {
            statements,
            final_expr,
        }
    }

    fn lower_stmt(&mut self, stmt: &Stmt) -> HirStmt {
        match stmt {
            Stmt::VarDecl { is_const, name, type_ann, init, .. } => HirStmt::VarDecl {
                is_const: *is_const,
                name: name.clone(),
                type_ann: type_ann.clone(),
                init: init.as_ref().map(|e| self.lower_expr(e)),
            },
            Stmt::Return { value, .. } => HirStmt::Return(value.as_ref().map(|e| self.lower_expr(e))),
            Stmt::Defer { body, .. } => HirStmt::Defer(self.lower_expr(body)),
            Stmt::Expr { expr, .. } => HirStmt::Expr(self.lower_expr(expr)),
        }
    }

    pub fn lower_expr(&mut self, expr: &Expr) -> HirExpr {
        match expr {
            Expr::Literal { value, .. } => HirExpr::Literal(value.clone()),
            Expr::Identifier { name, .. } => HirExpr::VarRef(name.clone()),
            Expr::Binary { left, op, right, .. } => HirExpr::Binary {
                left: Box::new(self.lower_expr(left)),
                op: *op,
                right: Box::new(self.lower_expr(right)),
            },
            Expr::Unary { op, expr, .. } => HirExpr::Unary {
                op: *op,
                expr: Box::new(self.lower_expr(expr)),
            },
            Expr::Call { callee, args, .. } => {
                let callee_hir = self.lower_expr(callee);
                let args_hir: Vec<HirExpr> = args.iter().map(|a| self.lower_expr(a)).collect();

                // Canonicalize borrow(...) and move(...) built-ins
                if let HirExpr::VarRef(ref name) = callee_hir {
                    if name == "borrow" && args_hir.len() == 1 {
                        return HirExpr::Borrow(Box::new(args_hir[0].clone()));
                    } else if name == "move" && args_hir.len() == 1 {
                        return HirExpr::Move(Box::new(args_hir[0].clone()));
                    }
                }

                HirExpr::Call {
                    callee: Box::new(callee_hir),
                    args: args_hir,
                }
            }
            Expr::IntrinsicCall { name, args, .. } => {
                let args_hir: Vec<HirExpr> = args.iter().map(|a| self.lower_expr(a)).collect();
                if name == "@borrow" && !args_hir.is_empty() {
                    HirExpr::Borrow(Box::new(args_hir[0].clone()))
                } else if name == "@move" && !args_hir.is_empty() {
                    HirExpr::Move(Box::new(args_hir[0].clone()))
                } else {
                    HirExpr::VarRef(name.clone())
                }
            }
            Expr::MemberAccess { object, property, is_optional, .. } => {
                let obj_hir = self.lower_expr(object);
                // Canonicalize x.borrow() and x.move() method calls
                if property == "borrow" {
                    HirExpr::Borrow(Box::new(obj_hir))
                } else if property == "move" {
                    HirExpr::Move(Box::new(obj_hir))
                } else {
                    HirExpr::Member {
                        object: Box::new(obj_hir),
                        property: property.clone(),
                        is_optional: *is_optional,
                    }
                }
            }
            Expr::StructLiteral { name, fields, .. } => {
                let mut lowered_fields = Vec::new();
                for f in fields {
                    let val = match &f.value {
                        Some(v) => self.lower_expr(v),
                        // Desugar field shorthand: `User { name }` -> `User { name: name }`
                        None => HirExpr::VarRef(f.name.clone()),
                    };
                    lowered_fields.push((f.name.clone(), val));
                }
                HirExpr::StructInit {
                    struct_name: name.clone(),
                    fields: lowered_fields,
                }
            }
            Expr::If { cond, then_branch, else_branch, .. } => HirExpr::If {
                cond: Box::new(self.lower_expr(cond)),
                then_branch: self.lower_block(then_branch),
                else_branch: else_branch.as_ref().map(|e| Box::new(self.lower_expr(e))),
            },
            Expr::Match { value, arms, .. } => {
                let value_hir = self.lower_expr(value);
                let arms_hir = arms
                    .iter()
                    .map(|arm| HirMatchArm {
                        pattern: arm.pattern.clone(),
                        guard: arm.guard.as_ref().map(|g| self.lower_expr(g)),
                        body: self.lower_expr(&arm.body),
                    })
                    .collect();
                HirExpr::Match {
                    value: Box::new(value_hir),
                    arms: arms_hir,
                }
            }
            Expr::Block(b) => HirExpr::Block(self.lower_block(b)),
            Expr::ComptimeBlock { body, .. } => HirExpr::Comptime(self.lower_block(body)),
            Expr::SpawnBlock { body, .. } => HirExpr::Spawn(self.lower_block(body)),
            Expr::NullCoalesce { left, right, .. } => HirExpr::Binary {
                left: Box::new(self.lower_expr(left)),
                op: BinaryOp::Equal, // Desugars into null check in lowering
                right: Box::new(self.lower_expr(right)),
            },
        }
    }
}
