use crate::nodes::*;
use arca_ast::{BinaryOp, LiteralKind, Pattern};
use arca_hir::*;
use arca_typechecker::{PrimitiveType, Type};
use std::collections::HashMap;

#[derive(Clone)]
struct BlockBuilder {
    id: BlockId,
    instrs: Vec<AirInstruction>,
}

impl BlockBuilder {
    fn new(id: BlockId) -> Self {
        Self { id, instrs: Vec::new() }
    }
    fn push(&mut self, instr: AirInstruction) {
        self.instrs.push(instr);
    }
    fn finish(self, terminator: AirTerminator) -> BasicBlock {
        BasicBlock { id: self.id, instructions: self.instrs, terminator }
    }
}

struct LoopFrame {
    header_block: BlockId,
    exit_block: BlockId,
}

struct LoweringCtx {
    blocks: Vec<BasicBlock>,
    current: BlockBuilder,
    loop_stack: Vec<LoopFrame>,
    param_regs: Vec<RegisterId>,
    last_expr_value: Option<AirValue>,
    defer_stack: Vec<HirExpr>,
}

impl LoweringCtx {
    fn new(entry_id: BlockId) -> Self {
        Self {
            blocks: Vec::new(), current: BlockBuilder::new(entry_id),
            loop_stack: Vec::new(), param_regs: Vec::new(), last_expr_value: None,
            defer_stack: Vec::new(),
        }
    }

    fn push_loop(&mut self, header: BlockId, exit: BlockId) {
        self.loop_stack.push(LoopFrame { header_block: header, exit_block: exit });
    }

    fn pop_loop(&mut self) {
        self.loop_stack.pop();
    }

    fn loop_exit(&self) -> Option<BlockId> {
        self.loop_stack.last().map(|f| f.exit_block)
    }

    fn loop_header(&self) -> Option<BlockId> {
        self.loop_stack.last().map(|f| f.header_block)
    }

    fn set_terminator_and_switch(&mut self, terminator: AirTerminator, next_block: BlockId) {
        let new_current = BlockBuilder::new(next_block);
        let old = std::mem::replace(&mut self.current, new_current);
        self.blocks.push(old.finish(terminator));
    }

    fn finish_all(mut self, final_terminator: AirTerminator) -> Vec<BasicBlock> {
        if !self.current.instrs.is_empty() || self.blocks.is_empty() {
            self.blocks.push(self.current.finish(final_terminator));
        } else {
            self.blocks.last_mut().unwrap().terminator = final_terminator;
        }
        self.blocks
    }
}

pub struct AirBuilder {
    next_reg: u32,
    next_block: u32,
    known_struct_vars: HashMap<String, String>,
}

impl AirBuilder {
    pub fn new() -> Self {
        Self { next_reg: 0, next_block: 0, known_struct_vars: HashMap::new() }
    }

    fn fresh_reg(&mut self) -> RegisterId {
        let id = self.next_reg; self.next_reg += 1; RegisterId(id)
    }

    fn fresh_block(&mut self) -> BlockId {
        let id = self.next_block; self.next_block += 1; BlockId(id)
    }

    pub fn build_module(&mut self, hir: &HirProgram) -> AirModule {
        let mut functions = HashMap::new();
        for (name, hir_fn) in &hir.functions {
            functions.insert(name.clone(), self.build_function(hir_fn));
        }
        for hir_struct in hir.structs.values() {
            for (mname, mfn) in &hir_struct.methods {
                let mut method_fn = mfn.clone();
                if let Some(first_param) = method_fn.params.first_mut() {
                    if first_param.name == "self" {
                        first_param.type_ann = arca_ast::TypeAnnotation::Named(hir_struct.name.clone());
                    }
                }
                functions.insert(format!("{}.{}", hir_struct.name, mname), self.build_function(&method_fn));
            }
        }
        AirModule { name: "main_module".to_string(), functions }
    }

    pub fn build_function(&mut self, hir_fn: &HirFn) -> AirFunction {
        let entry_id = self.fresh_block();
        let mut var_map: HashMap<String, RegisterId> = HashMap::new();
        let mut ctx = LoweringCtx::new(entry_id);

        for param in &hir_fn.params {
            let reg = self.fresh_reg();
            ctx.param_regs.push(reg);
            var_map.insert(param.name.clone(), reg);
        }

        for stmt in &hir_fn.body.statements {
            self.lower_stmt(stmt, &mut ctx, &mut var_map);
        }

        let ret_val = hir_fn.body.final_expr.as_ref()
            .map(|e| self.lower_expr(e, &mut ctx, &mut var_map));

        let defers = std::mem::take(&mut ctx.defer_stack);
        for d in defers.into_iter().rev() {
            self.lower_expr(&d, &mut ctx, &mut var_map);
        }

        let return_type = hir_fn
            .return_type
            .as_ref()
            .map(|t| hir_type_to_air_type(t))
            .unwrap_or(Type::Primitive(PrimitiveType::Void));

        let air_params: Vec<(String, Type)> = hir_fn
            .params
            .iter()
            .map(|p| (p.name.clone(), hir_type_to_air_type(&p.type_ann)))
            .collect();

        let param_registers: Vec<RegisterId> = ctx.param_regs.clone();

        AirFunction {
            name: hir_fn.name.clone(),
            params: air_params,
            param_registers,
            return_type,
            blocks: ctx.finish_all(AirTerminator::Ret(ret_val)),
            entry_block: entry_id,
        }
    }

    fn lower_stmt(&mut self, stmt: &HirStmt, ctx: &mut LoweringCtx, var_map: &mut HashMap<String, RegisterId>) {
        match stmt {
            HirStmt::VarDecl { name, init, .. } => {
                let ptr_reg = self.fresh_reg();
                ctx.current.push(AirInstruction::Alloca { target: ptr_reg, ty: Type::Primitive(PrimitiveType::I64) });
                var_map.insert(name.clone(), ptr_reg);
                if let Some(init_expr) = init {
                    if let HirExpr::StructInit { struct_name, .. } = init_expr {
                        self.known_struct_vars.insert(name.clone(), struct_name.clone());
                    }
                    let val = self.lower_expr(init_expr, ctx, var_map);
                    ctx.current.push(AirInstruction::Store { ptr: ptr_reg, val });
                }
            }
            HirStmt::Return(opt_expr) => {
                let ret_val = opt_expr.as_ref().map(|e| self.lower_expr(e, ctx, var_map));
                let defers = std::mem::take(&mut ctx.defer_stack);
                for d in defers.into_iter().rev() {
                    self.lower_expr(&d, ctx, var_map);
                }
                let next = self.fresh_block();
                ctx.set_terminator_and_switch(AirTerminator::Ret(ret_val), next);
            }
            HirStmt::Expr(expr) => {
                let val = self.lower_expr(expr, ctx, var_map);
                ctx.last_expr_value = Some(val);
            }
            HirStmt::Defer(expr) => { ctx.defer_stack.push(expr.clone()); }
            HirStmt::Break => {
                if let Some(exit) = ctx.loop_exit() {
                    let next = self.fresh_block();
                    ctx.set_terminator_and_switch(AirTerminator::Br(exit), next);
                }
            }
            HirStmt::Continue => {
                if let Some(header) = ctx.loop_header() {
                    let next = self.fresh_block();
                    ctx.set_terminator_and_switch(AirTerminator::Br(header), next);
                }
            }
            HirStmt::Destructure { fields, init, .. } => {
                let init_val = self.lower_expr(init, ctx, var_map);
                for fname in fields {
                    let ptr_reg = self.fresh_reg();
                    ctx.current.push(AirInstruction::Alloca { target: ptr_reg, ty: Type::Primitive(PrimitiveType::I64) });
                    var_map.insert(fname.clone(), ptr_reg);
                    ctx.current.push(AirInstruction::Store { ptr: ptr_reg, val: init_val.clone() });
                }
            }
            HirStmt::Assign { target, value } => {
                let val = self.lower_expr(value, ctx, var_map);
                if let Some(&reg) = var_map.get(target) {
                    ctx.current.push(AirInstruction::Store { ptr: reg, val });
                }
            }
        }
    }

    fn lower_expr(&mut self, expr: &HirExpr, ctx: &mut LoweringCtx, var_map: &mut HashMap<String, RegisterId>) -> AirValue {
        match expr {
            HirExpr::Literal(lit) => match lit {
                LiteralKind::Int(n) => AirValue::ConstInt(*n),
                LiteralKind::Float(f) => AirValue::ConstFloat(*f),
                LiteralKind::String(s) => AirValue::ConstString(s.clone()),
                LiteralKind::Bool(b) => AirValue::ConstBool(*b),
                LiteralKind::Char(c) => AirValue::ConstString(c.to_string()),
                LiteralKind::None => AirValue::ConstInt(0),
            },
            HirExpr::VarRef(name) => {
                if let Some(reg) = var_map.get(name) {
                    if ctx.param_regs.contains(reg) {
                        AirValue::Register(*reg)
                    } else {
                        let loaded_reg = self.fresh_reg();
                        ctx.current.push(AirInstruction::Load { target: loaded_reg, ptr: *reg, ty: Type::Primitive(PrimitiveType::I64) });
                        AirValue::Register(loaded_reg)
                    }
                } else {
                    AirValue::ConstString(name.clone())
                }
            }
            HirExpr::Binary { left, op, right } => {
                let lval = self.lower_expr(left, ctx, var_map);
                let rval = self.lower_expr(right, ctx, var_map);
                let target = self.fresh_reg();
                ctx.current.push(AirInstruction::Binary { target, op: *op, left: lval, right: rval });
                AirValue::Register(target)
            }
            HirExpr::Unary { op, expr } => {
                let val = self.lower_expr(expr, ctx, var_map);
                let target = self.fresh_reg();
                let (left, right, air_op) = match op {
                    arca_ast::UnaryOp::Not => (val, AirValue::ConstBool(false), BinaryOp::Equal),
                    arca_ast::UnaryOp::Neg => (AirValue::ConstInt(0), val, BinaryOp::Sub),
                };
                ctx.current.push(AirInstruction::Binary { target, op: air_op, left, right });
                AirValue::Register(target)
            }
            HirExpr::Call { callee, args } => {
                let mut arg_vals = Vec::new();
                for a in args { arg_vals.push(self.lower_expr(a, ctx, var_map)); }
                let (callee_name, method_obj) = match &**callee {
                    HirExpr::VarRef(n) => (n.clone(), None),
                    HirExpr::Member { object, property, .. } => {
                        let obj_val = Some(self.lower_expr(object, ctx, var_map));
                        let fn_name = match &**object {
                            HirExpr::VarRef(n) => {
                                if let Some(sname) = self.known_struct_vars.get(n) {
                                    format!("{}.{}", sname, property)
                                } else {
                                    format!("{}.{}", n, property)
                                }
                            }
                            _ => property.clone(),
                        };
                        (fn_name, obj_val)
                    }
                    _ => ("unknown_callee".to_string(), None),
                };
                // Normalize built-in method calls to known runtime function names
                let (normalized_name, final_args) = self.normalize_call(&callee_name, method_obj, &arg_vals);
                let target = self.fresh_reg();
                ctx.current.push(AirInstruction::Call { target: Some(target), fn_name: normalized_name, args: final_args });
                AirValue::Register(target)
            }
            HirExpr::StructInit { struct_name, fields } => {
                let mut field_vals = Vec::new();
                for (fname, fexpr) in fields { field_vals.push((fname.clone(), self.lower_expr(fexpr, ctx, var_map))); }
                let target = self.fresh_reg();
                ctx.current.push(AirInstruction::StructInit { target, struct_name: struct_name.clone(), fields: field_vals });
                AirValue::Register(target)
            }
            HirExpr::Member { object, property, .. } => {
                let obj_val = self.lower_expr(object, ctx, var_map);
                let obj_reg = match obj_val {
                    AirValue::Register(r) => r,
                    _ => {
                        let r = self.fresh_reg();
                        ctx.current.push(AirInstruction::Alloca { target: r, ty: Type::Primitive(PrimitiveType::I64) });
                        ctx.current.push(AirInstruction::Store { ptr: r, val: obj_val });
                        r
                    }
                };
                let target = self.fresh_reg();
                ctx.current.push(AirInstruction::FieldLoad { target, object: obj_reg, field: property.clone() });
                AirValue::Register(target)
            }
            HirExpr::If { cond, then_branch, else_branch } =>
                self.lower_if(cond, then_branch, else_branch.as_deref(), ctx, var_map),
            HirExpr::Match { value, arms } => self.lower_match(value, arms, ctx, var_map),
            HirExpr::Loop(body) => self.lower_loop(body, ctx, var_map),
            HirExpr::Block(b) => {
                for stmt in &b.statements { self.lower_stmt(stmt, ctx, var_map); }
                b.final_expr.as_ref().map(|fe| self.lower_expr(fe, ctx, var_map))
                    .or_else(|| ctx.last_expr_value.take())
                    .unwrap_or(AirValue::ConstInt(0))
            }
            HirExpr::Borrow(inner) => {
                let inner_val = self.lower_expr(inner, ctx, var_map);
                let inner_reg = match inner_val {
                    AirValue::Register(r) => r,
                    _ => {
                        let r = self.fresh_reg();
                        ctx.current.push(AirInstruction::Alloca { target: r, ty: Type::Primitive(PrimitiveType::I64) });
                        ctx.current.push(AirInstruction::Store { ptr: r, val: inner_val });
                        r
                    }
                };
                let target = self.fresh_reg();
                ctx.current.push(AirInstruction::Ref { target, source: inner_reg });
                AirValue::Register(target)
            }
            HirExpr::Move(inner) => self.lower_expr(inner, ctx, var_map),
            HirExpr::Comptime(b) => {
                for stmt in &b.statements { self.lower_stmt(stmt, ctx, var_map); }
                b.final_expr.as_ref().map(|fe| self.lower_expr(fe, ctx, var_map)).unwrap_or(AirValue::ConstInt(0))
            }
            HirExpr::GroupBlock(b) => {
                for stmt in &b.statements { self.lower_stmt(stmt, ctx, var_map); }
                b.final_expr.as_ref().map(|fe| self.lower_expr(fe, ctx, var_map)).unwrap_or(AirValue::ConstInt(0))
            }
            HirExpr::Closure { body, .. } => self.lower_expr(body, ctx, var_map),
            HirExpr::TryBlock(b) => self.lower_try(b, ctx, var_map),
            HirExpr::Spawn(b) => self.lower_spawn(b, ctx, var_map),
            HirExpr::ForLoop { init, cond, update, body } => self.lower_for_loop(init.as_deref(), cond.as_deref(), update.as_deref(), body, ctx, var_map),
            HirExpr::ForIn { index_var, item_var, iterable, body } => self.lower_for_in(index_var, item_var, iterable, body, ctx, var_map),
            HirExpr::Throw(value) => {
                let val = self.lower_expr(value, ctx, var_map);
                let err_slot = self.fresh_reg();
                ctx.current.push(AirInstruction::Alloca { target: err_slot, ty: Type::Primitive(PrimitiveType::I64) });
                ctx.current.push(AirInstruction::Store { ptr: err_slot, val });
                let target = self.fresh_reg();
                ctx.current.push(AirInstruction::Call {
                    target: Some(target),
                    fn_name: "__arca_throw".to_string(),
                    args: vec![AirValue::Register(err_slot)],
                });
                AirValue::Register(target)
            }
        }
    }

    fn lower_try(&mut self, body: &HirBlock, ctx: &mut LoweringCtx, var_map: &mut HashMap<String, RegisterId>) -> AirValue {
        let body_block = self.fresh_block();
        let catch_block = self.fresh_block();
        let merge_block = self.fresh_block();
        let result_slot = self.fresh_reg();
        ctx.current.push(AirInstruction::Alloca { target: result_slot, ty: Type::Primitive(PrimitiveType::I64) });
        ctx.set_terminator_and_switch(AirTerminator::Br(body_block), body_block);

        let body_var_map = var_map.clone();
        for stmt in &body.statements { self.lower_stmt(stmt, ctx, var_map); }
        let body_val = body.final_expr.as_ref()
            .map(|fe| self.lower_expr(fe, ctx, var_map))
            .unwrap_or(AirValue::ConstInt(0));
        ctx.current.push(AirInstruction::Store { ptr: result_slot, val: body_val });
        ctx.set_terminator_and_switch(AirTerminator::Br(merge_block), catch_block);

        // Catch block: store error sentinel and fall through
        ctx.current.push(AirInstruction::Store { ptr: result_slot, val: AirValue::ConstInt(-1) });
        ctx.set_terminator_and_switch(AirTerminator::Br(merge_block), merge_block);

        let loaded = self.fresh_reg();
        ctx.current.push(AirInstruction::Load { target: loaded, ptr: result_slot, ty: Type::Primitive(PrimitiveType::I64) });
        AirValue::Register(loaded)
    }

    fn lower_spawn(&mut self, body: &HirBlock, ctx: &mut LoweringCtx, var_map: &mut HashMap<String, RegisterId>) -> AirValue {
        let spawn_block = self.fresh_block();
        ctx.set_terminator_and_switch(AirTerminator::Br(spawn_block), spawn_block);

        for stmt in &body.statements { self.lower_stmt(stmt, ctx, var_map); }
        let body_val = body.final_expr.as_ref()
            .map(|fe| self.lower_expr(fe, ctx, var_map))
            .unwrap_or(AirValue::ConstInt(0));

        let target = self.fresh_reg();
        ctx.current.push(AirInstruction::Call {
            target: Some(target),
            fn_name: "__arca_spawn".to_string(),
            args: vec![body_val],
        });
        AirValue::Register(target)
    }

    fn lower_if(&mut self, cond: &HirExpr, then_branch: &HirBlock, else_branch: Option<&HirExpr>,
                 ctx: &mut LoweringCtx, var_map: &mut HashMap<String, RegisterId>) -> AirValue {
        let cond_val = self.lower_expr(cond, ctx, var_map);
        let result_slot = self.fresh_reg();
        ctx.current.push(AirInstruction::Alloca { target: result_slot, ty: Type::Primitive(PrimitiveType::I64) });
        let then_block = self.fresh_block();
        let else_block = self.fresh_block();
        let merge_block = self.fresh_block();
        ctx.set_terminator_and_switch(AirTerminator::CondBr { cond: cond_val, then_block, else_block }, then_block);

        let then_var_map = var_map.clone();
        for stmt in &then_branch.statements { self.lower_stmt(stmt, ctx, var_map); }
        let then_val = then_branch.final_expr.as_ref()
            .map(|fe| self.lower_expr(fe, ctx, var_map))
            .unwrap_or(AirValue::ConstInt(0));
        ctx.current.push(AirInstruction::Store { ptr: result_slot, val: then_val });
        ctx.set_terminator_and_switch(AirTerminator::Br(merge_block), else_block);

        *var_map = then_var_map;
        if let Some(eb) = else_branch {
            match eb {
                HirExpr::Block(b) => {
                    for stmt in &b.statements { self.lower_stmt(stmt, ctx, var_map); }
                    let else_val = b.final_expr.as_ref()
                        .map(|fe| self.lower_expr(fe, ctx, var_map))
                        .unwrap_or(AirValue::ConstInt(0));
                    ctx.current.push(AirInstruction::Store { ptr: result_slot, val: else_val });
                }
                _ => {
                    let else_val = self.lower_expr(eb, ctx, var_map);
                    ctx.current.push(AirInstruction::Store { ptr: result_slot, val: else_val });
                }
            }
        } else {
            ctx.current.push(AirInstruction::Store { ptr: result_slot, val: AirValue::ConstInt(0) });
        }
        ctx.set_terminator_and_switch(AirTerminator::Br(merge_block), merge_block);
        let loaded = self.fresh_reg();
        ctx.current.push(AirInstruction::Load { target: loaded, ptr: result_slot, ty: Type::Primitive(PrimitiveType::I64) });
        AirValue::Register(loaded)
    }

    fn lower_match(&mut self, value: &HirExpr, arms: &[HirMatchArm],
                    ctx: &mut LoweringCtx, var_map: &mut HashMap<String, RegisterId>) -> AirValue {
        let match_val = self.lower_expr(value, ctx, var_map);
        let result_slot = self.fresh_reg();
        ctx.current.push(AirInstruction::Alloca { target: result_slot, ty: Type::Primitive(PrimitiveType::I64) });
        let merge_block = self.fresh_block();
        if arms.is_empty() {
            ctx.current.push(AirInstruction::Store { ptr: result_slot, val: AirValue::ConstInt(0) });
            ctx.set_terminator_and_switch(AirTerminator::Br(merge_block), merge_block);
        } else {
            let arm_blocks: Vec<BlockId> = (0..arms.len()).map(|_| self.fresh_block()).collect();
            ctx.set_terminator_and_switch(AirTerminator::Br(arm_blocks[0]), arm_blocks[0]);
            for (i, arm) in arms.iter().enumerate() {
                let next = if i + 1 < arm_blocks.len() { arm_blocks[i + 1] } else { merge_block };
                if let Pattern::Identifier(name) = &arm.pattern {
                    if name != "_" {
                        let ptr_reg = self.fresh_reg();
                        ctx.current.push(AirInstruction::Alloca { target: ptr_reg, ty: Type::Primitive(PrimitiveType::I64) });
                        ctx.current.push(AirInstruction::Store { ptr: ptr_reg, val: match_val.clone() });
                        var_map.insert(name.clone(), ptr_reg);
                    }
                }
                let arm_val = self.lower_expr(&arm.body, ctx, var_map);
                ctx.current.push(AirInstruction::Store { ptr: result_slot, val: arm_val });
                ctx.set_terminator_and_switch(AirTerminator::Br(next), next);
            }
        }
        let loaded = self.fresh_reg();
        ctx.current.push(AirInstruction::Load { target: loaded, ptr: result_slot, ty: Type::Primitive(PrimitiveType::I64) });
        AirValue::Register(loaded)
    }

    /// Normalize method calls to well-known runtime function names.
    /// Maps x.to_string() → __arca_int_to_str(x), req.path.starts_with(p) → __arca_starts_with(req.path, p), etc.
    fn normalize_call(&mut self, callee_name: &str, method_obj: Option<AirValue>, args: &[AirValue]) -> (String, Vec<AirValue>) {
        // Helper to check if callee matches a method name (with or without object prefix)
        let is_method = |name: &str| -> bool {
            callee_name == name || callee_name.ends_with(&format!(".{}", name))
        };

        if is_method("to_string") {
            let mut new_args = Vec::new();
            if let Some(obj) = method_obj { new_args.push(obj); }
            new_args.extend_from_slice(args);
            return ("__arca_int_to_str".to_string(), new_args);
        }
        if is_method("starts_with") {
            let mut new_args = Vec::new();
            if let Some(obj) = method_obj { new_args.push(obj); }
            new_args.extend_from_slice(args);
            return ("__arca_starts_with".to_string(), new_args);
        }
        if is_method("parse_int") {
            let mut new_args = Vec::new();
            if let Some(obj) = method_obj { new_args.push(obj); }
            new_args.extend_from_slice(args);
            return ("__arca_parse_int".to_string(), new_args);
        }
        if is_method("rfind") {
            let mut new_args = Vec::new();
            if let Some(obj) = method_obj { new_args.push(obj); }
            new_args.extend_from_slice(args);
            return ("__arca_str_rfind".to_string(), new_args);
        }
        if is_method("slice") {
            let mut new_args = Vec::new();
            if let Some(obj) = method_obj { new_args.push(obj); }
            new_args.extend_from_slice(args);
            return ("__arca_str_slice".to_string(), new_args);
        }
        if callee_name.ends_with("elapsed_ms") || callee_name.ends_with("elapsed_ns") {
            let mut new_args = Vec::new();
            if let Some(obj) = method_obj { new_args.push(obj); }
            new_args.extend_from_slice(args);
            return (callee_name.to_string(), new_args);
        }
        let mut final_args = Vec::new();
        if let Some(obj) = method_obj {
            final_args.push(obj);
        }
        final_args.extend_from_slice(args);
        (callee_name.to_string(), final_args)
    }

    fn lower_for_loop(&mut self, init: Option<&HirStmt>, cond: Option<&HirExpr>, update: Option<&HirStmt>,
                       body: &HirBlock, ctx: &mut LoweringCtx, var_map: &mut HashMap<String, RegisterId>) -> AirValue {
        let header_block = self.fresh_block();
        let _cond_block = self.fresh_block();
        let body_block = self.fresh_block();
        let update_block = self.fresh_block();
        let exit_block = self.fresh_block();

        // Init
        if let Some(init_stmt) = init {
            self.lower_stmt(init_stmt, ctx, var_map);
        }
        ctx.set_terminator_and_switch(AirTerminator::Br(header_block), header_block);

        // Header: cond check
        let cond_val = if let Some(c) = cond {
            self.lower_expr(c, ctx, var_map)
        } else {
            AirValue::ConstBool(true)
        };
        ctx.set_terminator_and_switch(
            AirTerminator::CondBr { cond: cond_val, then_block: body_block, else_block: exit_block },
            body_block,
        );

        // Body
        ctx.push_loop(update_block, exit_block);
        let body_var_map = var_map.clone();
        for stmt in &body.statements { self.lower_stmt(stmt, ctx, var_map); }
        if let Some(ref fe) = body.final_expr { self.lower_expr(fe, ctx, var_map); }
        ctx.set_terminator_and_switch(AirTerminator::Br(update_block), update_block);

        // Update
        if let Some(u) = update { self.lower_stmt(u, ctx, var_map); }
        ctx.set_terminator_and_switch(AirTerminator::Br(header_block), exit_block);
        ctx.pop_loop();
        *var_map = body_var_map;
        AirValue::ConstInt(0)
    }

    fn lower_for_in(&mut self, index_var: &Option<String>, item_var: &str, iterable: &HirExpr,
                     body: &HirBlock, ctx: &mut LoweringCtx, var_map: &mut HashMap<String, RegisterId>) -> AirValue {
        let _iter_val = self.lower_expr(iterable, ctx, var_map);
        let _header_block = self.fresh_block();
        let _body_block = self.fresh_block();
        let _exit_block = self.fresh_block();

        let _idx_ptr = self.fresh_reg();
        ctx.current.push(AirInstruction::Alloca { target: _idx_ptr, ty: Type::Primitive(PrimitiveType::I64) });
        ctx.current.push(AirInstruction::Call {
            target: Some(_idx_ptr),
            fn_name: "__arca_for_in_iter".to_string(),
            args: vec![],
        });

        let item_ptr = self.fresh_reg();
        ctx.current.push(AirInstruction::Alloca { target: item_ptr, ty: Type::Primitive(PrimitiveType::I64) });
        var_map.insert(item_var.to_string(), item_ptr);

        if let Some(idx_var) = index_var {
            let idx_ptr = self.fresh_reg();
            ctx.current.push(AirInstruction::Alloca { target: idx_ptr, ty: Type::Primitive(PrimitiveType::I64) });
            var_map.insert(idx_var.clone(), idx_ptr);
        }

        for stmt in &body.statements { self.lower_stmt(stmt, ctx, var_map); }
        if let Some(ref fe) = body.final_expr { self.lower_expr(fe, ctx, var_map); }
        AirValue::ConstInt(0)
    }

    fn lower_loop(&mut self, body: &HirBlock, ctx: &mut LoweringCtx, var_map: &mut HashMap<String, RegisterId>) -> AirValue {
        let header_block = self.fresh_block();
        let body_block = self.fresh_block();
        let exit_block = self.fresh_block();

        ctx.push_loop(header_block, exit_block);
        ctx.set_terminator_and_switch(AirTerminator::Br(header_block), header_block);
        ctx.set_terminator_and_switch(
            AirTerminator::CondBr { cond: AirValue::ConstBool(true), then_block: body_block, else_block: exit_block },
            body_block,
        );

        let body_var_map = var_map.clone();
        for stmt in &body.statements { self.lower_stmt(stmt, ctx, var_map); }
        if let Some(ref fe) = body.final_expr { self.lower_expr(fe, ctx, var_map); }
        ctx.set_terminator_and_switch(AirTerminator::Br(header_block), exit_block);
        ctx.pop_loop();
        *var_map = body_var_map;
        AirValue::ConstInt(0)
    }
}

fn hir_type_to_air_type(ann: &arca_ast::TypeAnnotation) -> Type {
    match ann {
        arca_ast::TypeAnnotation::Named(name) => match name.as_str() {
            "i8" => Type::Primitive(PrimitiveType::I8),
            "i16" => Type::Primitive(PrimitiveType::I16),
            "i32" => Type::Primitive(PrimitiveType::I32),
            "i64" | "int" => Type::Primitive(PrimitiveType::I64),
            "u8" => Type::Primitive(PrimitiveType::U8),
            "u16" => Type::Primitive(PrimitiveType::U16),
            "u32" => Type::Primitive(PrimitiveType::U32),
            "u64" => Type::Primitive(PrimitiveType::U64),
            "f32" => Type::Primitive(PrimitiveType::F32),
            "f64" => Type::Primitive(PrimitiveType::F64),
            "bool" => Type::Primitive(PrimitiveType::Bool),
            "string" => Type::Primitive(PrimitiveType::String),
            "void" => Type::Primitive(PrimitiveType::Void),
            custom => Type::Struct {
                name: custom.to_string(),
                fields: std::collections::HashMap::new(),
                methods: std::collections::HashMap::new(),
            },
        },
        _ => Type::Primitive(PrimitiveType::I64),
    }
}
