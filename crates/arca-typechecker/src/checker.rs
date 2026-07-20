//! Type Checker pass operating on HIR.

use crate::env::TypeEnv;
use crate::types::{FnType, PrimitiveType, Type};
use arca_ast::{BinaryOp, LiteralKind, TypeAnnotation, UnaryOp};
use arca_diagnostics::Diagnostic;
use arca_hir::*;
use std::collections::HashMap;

pub struct TypeChecker {
    env: TypeEnv,
    diagnostics: Vec<Diagnostic>,
}

impl TypeChecker {
    pub fn new() -> Self {
        Self {
            env: TypeEnv::new(),
            diagnostics: Vec::new(),
        }
    }

    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    pub fn check_program(&mut self, hir: &HirProgram) -> Vec<Diagnostic> {
        // Register struct types
        for (name, hir_struct) in &hir.structs {
            let mut field_types = HashMap::new();
            for f in &hir_struct.fields {
                let ty = self.resolve_ast_type(&f.type_ann);
                field_types.insert(f.name.clone(), ty);
            }

            let mut method_types = HashMap::new();
            for (mname, mfn) in &hir_struct.methods {
                let param_types = mfn
                    .params
                    .iter()
                    .map(|p| self.resolve_ast_type(&p.type_ann))
                    .collect();
                let ret_type = mfn
                    .return_type
                    .as_ref()
                    .map(|r| self.resolve_ast_type(r))
                    .unwrap_or(Type::Primitive(PrimitiveType::Void));

                method_types.insert(
                    mname.clone(),
                    FnType {
                        params: param_types,
                        return_type: Box::new(ret_type),
                    },
                );
            }

            let st_type = Type::Struct {
                name: name.clone(),
                fields: field_types,
                methods: method_types,
            };

            self.env.structs.insert(name.clone(), st_type);
        }

        // Register top-level functions
        for (name, hir_fn) in &hir.functions {
            let param_types = hir_fn
                .params
                .iter()
                .map(|p| self.resolve_ast_type(&p.type_ann))
                .collect();
            let ret_type = hir_fn
                .return_type
                .as_ref()
                .map(|r| self.resolve_ast_type(r))
                .unwrap_or(Type::Primitive(PrimitiveType::Void));

            self.env.functions.insert(
                name.clone(),
                FnType {
                    params: param_types,
                    return_type: Box::new(ret_type),
                },
            );
        }

        // Check struct methods
        for hir_struct in hir.structs.values() {
            let st_ty = &self.env.structs.get(&hir_struct.name).cloned();
            for mfn in hir_struct.methods.values() {
                self.env.push_scope();

                // Bind struct fields in method scope
                if let Some(Type::Struct { fields, .. }) = st_ty {
                    for (fname, fty) in fields {
                        self.env.insert_var(fname.clone(), fty.clone());
                    }
                }

                self.check_fn_body(mfn);
                self.env.pop_scope();
            }
        }

        // Check top-level functions
        for hir_fn in hir.functions.values() {
            self.check_fn(hir_fn);
        }

        std::mem::take(&mut self.diagnostics)
    }

    fn resolve_ast_type(&self, ann: &TypeAnnotation) -> Type {
        match ann {
            TypeAnnotation::Named(name) => self.env.lookup_type_annotation(name),
            TypeAnnotation::Generic { name, .. } => self.env.lookup_type_annotation(name),
            TypeAnnotation::Reference { is_mut, inner } => Type::Reference {
                is_mut: *is_mut,
                inner: Box::new(self.resolve_ast_type(inner)),
            },
            TypeAnnotation::Fn { params, return_type } => Type::Fn(FnType {
                params: params.iter().map(|p| self.resolve_ast_type(p)).collect(),
                return_type: Box::new(self.resolve_ast_type(return_type)),
            }),
        }
    }

    fn check_fn(&mut self, hir_fn: &HirFn) {
        self.env.push_scope();
        self.check_fn_body(hir_fn);
        self.env.pop_scope();
    }

    fn check_fn_body(&mut self, hir_fn: &HirFn) {
        for param in &hir_fn.params {
            let ptype = self.resolve_ast_type(&param.type_ann);
            self.env.insert_var(param.name.clone(), ptype);
        }

        let expected_ret = hir_fn
            .return_type
            .as_ref()
            .map(|r| self.resolve_ast_type(r))
            .unwrap_or(Type::Primitive(PrimitiveType::Void));

        self.check_block(&hir_fn.body, &expected_ret);
    }

    fn check_block(&mut self, block: &HirBlock, expected_ret: &Type) {
        self.env.push_scope();

        for stmt in &block.statements {
            self.check_stmt(stmt, expected_ret);
        }

        if let Some(final_expr) = &block.final_expr {
            let final_ty = self.infer_expr(final_expr);
            if !final_ty.is_assignable_to(expected_ret) {
                self.diagnostics.push(Diagnostic::error(format!(
                    "Type mismatch: expected '{}', found '{}'",
                    expected_ret, final_ty
                )));
            }
        }

        self.env.pop_scope();
    }

    fn check_stmt(&mut self, stmt: &HirStmt, expected_ret: &Type) {
        match stmt {
            HirStmt::VarDecl {
                is_const: _,
                name,
                type_ann,
                init,
            } => {
                let declared_type = type_ann.as_ref().map(|ann| self.resolve_ast_type(ann));

                let inferred_type = if let Some(init_expr) = init {
                    let init_ty = self.infer_expr(init_expr);
                    if let Some(ref decl_ty) = declared_type {
                        if !init_ty.is_assignable_to(decl_ty) {
                            self.diagnostics.push(Diagnostic::error(format!(
                                "Cannot assign type '{}' to variable '{}' of type '{}'",
                                init_ty, name, decl_ty
                            )));
                        }
                        decl_ty.clone()
                    } else {
                        match init_ty {
                            Type::UntypedInt(_) => Type::Primitive(PrimitiveType::I32),
                            Type::UntypedFloat(_) => Type::Primitive(PrimitiveType::F64),
                            other => other,
                        }
                    }
                } else {
                    declared_type.unwrap_or(Type::Unknown)
                };

                self.env.insert_var(name.clone(), inferred_type);
            }
            HirStmt::Return(opt_expr) => {
                let actual_ret = if let Some(expr) = opt_expr {
                    self.infer_expr(expr)
                } else {
                    Type::Primitive(PrimitiveType::Void)
                };

                if !actual_ret.is_assignable_to(expected_ret) {
                    self.diagnostics.push(Diagnostic::error(format!(
                        "Return type mismatch: expected '{}', found '{}'",
                        expected_ret, actual_ret
                    )));
                }
            }
            HirStmt::Defer(expr) => {
                self.infer_expr(expr);
            }
            HirStmt::Expr(expr) => {
                self.infer_expr(expr);
            }
        }
    }

    pub fn infer_expr(&mut self, expr: &HirExpr) -> Type {
        match expr {
            HirExpr::Literal(lit) => match lit {
                LiteralKind::Int(n) => Type::UntypedInt(*n),
                LiteralKind::Float(f) => Type::UntypedFloat(*f),
                LiteralKind::String(_) => Type::Primitive(PrimitiveType::String),
                LiteralKind::Char(_) => Type::Primitive(PrimitiveType::Char),
                LiteralKind::Bool(_) => Type::Primitive(PrimitiveType::Bool),
            },
            HirExpr::VarRef(name) => {
                if let Some(ty) = self.env.lookup_var(name) {
                    ty
                } else if let Some(st) = self.env.structs.get(name) {
                    st.clone()
                } else if let Some(fnt) = self.env.functions.get(name) {
                    Type::Fn(fnt.clone())
                } else {
                    self.diagnostics.push(Diagnostic::error(format!(
                        "Unknown variable or identifier '{}'",
                        name
                    )));
                    Type::Unknown
                }
            }
            HirExpr::Binary { left, op, right } => {
                let lty = self.infer_expr(left);
                let rty = self.infer_expr(right);

                match op {
                    BinaryOp::Equal
                    | BinaryOp::NotEqual
                    | BinaryOp::Less
                    | BinaryOp::LessEqual
                    | BinaryOp::Greater
                    | BinaryOp::GreaterEqual
                    | BinaryOp::And
                    | BinaryOp::Or => Type::Primitive(PrimitiveType::Bool),

                    BinaryOp::Add | BinaryOp::Sub | BinaryOp::Mul | BinaryOp::Div | BinaryOp::Rem => {
                        if lty.is_numeric() {
                            lty
                        } else {
                            rty
                        }
                    }
                }
            }
            HirExpr::Unary { op, expr } => {
                let ty = self.infer_expr(expr);
                match op {
                    UnaryOp::Not => Type::Primitive(PrimitiveType::Bool),
                    UnaryOp::Neg => ty,
                }
            }
            HirExpr::Call { callee, args } => {
                let callee_ty = self.infer_expr(callee);
                let arg_types: Vec<Type> = args.iter().map(|a| self.infer_expr(a)).collect();

                match callee_ty {
                    Type::Fn(fnt) => {
                        // Allow variadic/built-in functions like println, print, etc.
                        if fnt.params.is_empty() {
                            // Built-in variadic function
                        } else if arg_types.len() != fnt.params.len() {
                            self.diagnostics.push(Diagnostic::error(format!(
                                "Function call argument count mismatch: expected {}, found {}",
                                fnt.params.len(),
                                arg_types.len()
                            )));
                        } else {
                            for (idx, (arg_ty, param_ty)) in
                                arg_types.iter().zip(fnt.params.iter()).enumerate()
                            {
                                if !arg_ty.is_assignable_to(param_ty) {
                                    self.diagnostics.push(Diagnostic::error(format!(
                                        "Argument {} type mismatch: expected '{}', found '{}'",
                                        idx + 1,
                                        param_ty,
                                        arg_ty
                                    )));
                                }
                            }
                        }
                        *fnt.return_type
                    }
                    Type::Struct { name, methods, .. } => {
                        if let Some(new_m) = methods.get("new") {
                            *new_m.return_type.clone()
                        } else {
                            Type::Struct {
                                name,
                                fields: HashMap::new(),
                                methods: HashMap::new(),
                            }
                        }
                    }
                    _ => Type::Unknown,
                }
            }
            HirExpr::Member { object, property, .. } => {
                let obj_ty = self.infer_expr(object);
                match obj_ty {
                    Type::Struct { fields, methods, .. } => {
                        if let Some(fty) = fields.get(property) {
                            fty.clone()
                        } else if let Some(mty) = methods.get(property) {
                            Type::Fn(mty.clone())
                        } else {
                            self.diagnostics.push(Diagnostic::error(format!(
                                "Member property '{}' does not exist on struct",
                                property
                            )));
                            Type::Unknown
                        }
                    }
                    _ => Type::Unknown,
                }
            }
            HirExpr::StructInit {
                struct_name,
                fields,
            } => {
                if let Some(st_ty) = self.env.structs.get(struct_name).cloned() {
                    if let Type::Struct {
                        fields: expected_fields,
                        ..
                    } = &st_ty
                    {
                        for (fname, fexpr) in fields {
                            let fval_ty = self.infer_expr(fexpr);
                            if let Some(expected_ty) = expected_fields.get(fname) {
                                if !fval_ty.is_assignable_to(expected_ty) {
                                    self.diagnostics.push(Diagnostic::error(format!(
                                        "Field '{}' type mismatch in struct '{}': expected '{}', found '{}'",
                                        fname, struct_name, expected_ty, fval_ty
                                    )));
                                }
                            } else {
                                self.diagnostics.push(Diagnostic::error(format!(
                                    "Unknown field '{}' in struct '{}' initialization",
                                    fname, struct_name
                                )));
                            }
                        }
                    }
                    st_ty
                } else {
                    self.diagnostics.push(Diagnostic::error(format!(
                        "Unknown struct type '{}'",
                        struct_name
                    )));
                    Type::Unknown
                }
            }
            HirExpr::If {
                cond,
                then_branch,
                else_branch,
            } => {
                let cond_ty = self.infer_expr(cond);
                if !cond_ty.is_assignable_to(&Type::Primitive(PrimitiveType::Bool)) {
                    self.diagnostics.push(Diagnostic::error(format!(
                        "If condition must be of type 'bool', found '{}'",
                        cond_ty
                    )));
                }

                let then_ty = then_branch
                    .final_expr
                    .as_ref()
                    .map(|e| self.infer_expr(e))
                    .unwrap_or(Type::Primitive(PrimitiveType::Void));

                if let Some(else_expr) = else_branch {
                    let else_ty = self.infer_expr(else_expr);
                    if !then_ty.is_assignable_to(&else_ty) {
                        self.diagnostics.push(Diagnostic::error(format!(
                            "If/else branch type mismatch: 'then' has type '{}', 'else' has type '{}'",
                            then_ty, else_ty
                        )));
                    }
                }

                then_ty
            }
            HirExpr::Match { value, arms } => {
                let val_ty = self.infer_expr(value);
                let mut first_arm_ty = Type::Unknown;

                for (idx, arm) in arms.iter().enumerate() {
                    let arm_body_ty = self.infer_expr(&arm.body);
                    if idx == 0 {
                        first_arm_ty = arm_body_ty;
                    } else if !arm_body_ty.is_assignable_to(&first_arm_ty) {
                        self.diagnostics.push(Diagnostic::error(format!(
                            "Match arm type mismatch: expected '{}', found '{}'",
                            first_arm_ty, arm_body_ty
                        )));
                    }
                }

                let _ = val_ty;
                first_arm_ty
            }
            HirExpr::Block(b) => b
                .final_expr
                .as_ref()
                .map(|e| self.infer_expr(e))
                .unwrap_or(Type::Primitive(PrimitiveType::Void)),
            HirExpr::Borrow(inner) => {
                let inner_ty = self.infer_expr(inner);
                Type::Reference {
                    is_mut: false,
                    inner: Box::new(inner_ty),
                }
            }
            HirExpr::Move(inner) => self.infer_expr(inner),
            HirExpr::Comptime(b) => b
                .final_expr
                .as_ref()
                .map(|e| self.infer_expr(e))
                .unwrap_or(Type::Primitive(PrimitiveType::Void)),
            HirExpr::Spawn(b) => b
                .final_expr
                .as_ref()
                .map(|e| self.infer_expr(e))
                .unwrap_or(Type::Primitive(PrimitiveType::Void)),
        }
    }
}
