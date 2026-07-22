//! Borrow Checker and Ownership Analyzer operating on HIR.

use crate::state::OwnershipTracker;
use arca_diagnostics::Diagnostic;
use arca_hir::*;

#[derive(Debug, Default)]
pub struct BorrowChecker {
    tracker: OwnershipTracker,
    diagnostics: Vec<Diagnostic>,
}

impl BorrowChecker {
    pub fn new() -> Self {
        Self {
            tracker: OwnershipTracker::new(),
            diagnostics: Vec::new(),
        }
    }

    pub fn check_program(&mut self, hir: &HirProgram) -> Vec<Diagnostic> {
        for hir_fn in hir.functions.values() {
            self.check_fn(hir_fn);
        }

        for hir_struct in hir.structs.values() {
            for mfn in hir_struct.methods.values() {
                self.check_fn(mfn);
            }
        }

        std::mem::take(&mut self.diagnostics)
    }

    fn check_fn(&mut self, hir_fn: &HirFn) {
        self.tracker.push_scope();

        for param in &hir_fn.params {
            self.tracker.declare_var(param.name.clone());
        }

        self.check_block(&hir_fn.body);

        let _drops = self.tracker.pop_scope();
    }

    fn check_block(&mut self, block: &HirBlock) {
        self.tracker.push_scope();

        for stmt in &block.statements {
            self.check_stmt(stmt);
        }

        if let Some(final_expr) = &block.final_expr {
            self.check_expr(final_expr);
        }

        let _drops = self.tracker.pop_scope();
    }

    fn check_stmt(&mut self, stmt: &HirStmt) {
        match stmt {
            HirStmt::VarDecl { name, init, .. } => {
                if let Some(init_expr) = init {
                    self.check_expr(init_expr);
                }
                self.tracker.declare_var(name.clone());
            }
            HirStmt::Return(opt_expr) => {
                if let Some(expr) = opt_expr {
                    self.check_expr(expr);
                }
            }
            HirStmt::Defer(expr) => {
                self.check_expr(expr);
            }
            HirStmt::Expr(expr) => {
                self.check_expr(expr);
            }
            HirStmt::Break | HirStmt::Continue => {}
            HirStmt::Destructure { fields, init, .. } => {
                self.check_expr(init);
                for f in fields {
                    self.tracker.declare_var(f.clone());
                }
            }
            HirStmt::Assign { target: _, value } => {
                self.check_expr(value);
            }
        }
    }

    fn check_expr(&mut self, expr: &HirExpr) {
        match expr {
            HirExpr::VarRef(name) => {
                if let Err(msg) = self.tracker.check_used(name) {
                    self.diagnostics.push(Diagnostic::error(msg));
                }
            }
            HirExpr::Move(inner) => {
                if let HirExpr::VarRef(ref name) = **inner {
                    if !self.tracker.mark_moved(name) {
                        self.diagnostics.push(Diagnostic::error(format!(
                            "Use of moved value '{}'. Value was already moved.",
                            name
                        )));
                    }
                } else {
                    self.check_expr(inner);
                }
            }
            HirExpr::Borrow(inner) => {
                if let HirExpr::VarRef(ref name) = **inner {
                    if let Err(msg) = self.tracker.borrow_var(name, false) {
                        self.diagnostics.push(Diagnostic::error(msg));
                    }
                } else {
                    self.check_expr(inner);
                }
            }
            HirExpr::Binary { left, right, .. } => {
                self.check_expr(left);
                self.check_expr(right);
            }
            HirExpr::Unary { expr, .. } => {
                self.check_expr(expr);
            }
            HirExpr::Call { callee, args } => {
                self.check_expr(callee);
                for arg in args {
                    self.check_expr(arg);
                }
            }
            HirExpr::StructInit { fields, .. } => {
                for (_, fexpr) in fields {
                    self.check_expr(fexpr);
                }
            }
            HirExpr::If {
                cond,
                then_branch,
                else_branch,
            } => {
                self.check_expr(cond);
                self.check_block(then_branch);
                if let Some(else_expr) = else_branch {
                    self.check_expr(else_expr);
                }
            }
            HirExpr::Block(b) => {
                self.check_block(b);
            }
            HirExpr::Comptime(b) => {
                self.check_block(b);
            }
            HirExpr::GroupBlock(b) => {
                self.check_block(b);
            }
            HirExpr::Closure { params, body } => {
                for p in params {
                    self.tracker.declare_var(p.name.clone());
                }
                self.check_expr(body);
            }
            HirExpr::TryBlock(b) => {
                self.check_block(b);
            }
            HirExpr::Spawn(b) => {
                self.check_block(b);
            }
            _ => {}
        }
    }
}
