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
}

impl LoweringCtx {
    fn new(entry_id: BlockId) -> Self {
        Self {
            blocks: Vec::new(), current: BlockBuilder::new(entry_id),
            loop_stack: Vec::new(), param_regs: Vec::new(), last_expr_value: None,
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
}

impl AirBuilder {
    pub fn new() -> Self {
        Self { next_reg: 0, next_block: 0 }
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
                functions.insert(format!("{}.{}", hir_struct.name, mname), self.build_function(mfn));
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
                ctx.current.push(AirInstruction::Alloca { target: ptr_reg, ty: Type::Primitive(PrimitiveType::I32) });
                var_map.insert(name.clone(), ptr_reg);
                if let Some(init_expr) = init {
                    let val = self.lower_expr(init_expr, ctx, var_map);
                    ctx.current.push(AirInstruction::Store { ptr: ptr_reg, val });
                }
            }
            HirStmt::Return(opt_expr) => {
                let ret_val = opt_expr.as_ref().map(|e| self.lower_expr(e, ctx, var_map));
                let next = self.fresh_block();
                ctx.set_terminator_and_switch(AirTerminator::Ret(ret_val), next);
            }
            HirStmt::Expr(expr) => {
                let val = self.lower_expr(expr, ctx, var_map);
                ctx.last_expr_value = Some(val);
            }
            HirStmt::Defer(expr) => { self.lower_expr(expr, ctx, var_map); }
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
                    ctx.current.push(AirInstruction::Alloca { target: ptr_reg, ty: Type::Primitive(PrimitiveType::I32) });
                    var_map.insert(fname.clone(), ptr_reg);
                    ctx.current.push(AirInstruction::Store { ptr: ptr_reg, val: init_val.clone() });
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
                LiteralKind::Null => AirValue::ConstInt(0),
            },
            HirExpr::VarRef(name) => {
                if let Some(reg) = var_map.get(name) {
                    if ctx.param_regs.contains(reg) {
                        AirValue::Register(*reg)
                    } else {
                        let loaded_reg = self.fresh_reg();
                        ctx.current.push(AirInstruction::Load { target: loaded_reg, ptr: *reg, ty: Type::Primitive(PrimitiveType::I32) });
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
                        match &**object {
                            HirExpr::VarRef(n) => (format!("{}.{}", n, property), obj_val),
                            _ => (property.clone(), obj_val),
                        }
                    }
                    _ => ("unknown_callee".to_string(), None),
                };
                // Pass timer object as first arg for elapsed methods
                let final_args = if callee_name.ends_with("elapsed_ms") || callee_name.ends_with("elapsed_ns") {
                    if let Some(obj) = method_obj {
                        let mut with_obj = vec![obj];
                        with_obj.extend(arg_vals);
                        with_obj
                    } else { arg_vals }
                } else { arg_vals };
                let target = self.fresh_reg();
                ctx.current.push(AirInstruction::Call { target: Some(target), fn_name: callee_name, args: final_args });
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
                        ctx.current.push(AirInstruction::Alloca { target: r, ty: Type::Primitive(PrimitiveType::I32) });
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
                        ctx.current.push(AirInstruction::Alloca { target: r, ty: Type::Primitive(PrimitiveType::I32) });
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
            HirExpr::Throw(value) => {
                let val = self.lower_expr(value, ctx, var_map);
                let err_slot = self.fresh_reg();
                ctx.current.push(AirInstruction::Alloca { target: err_slot, ty: Type::Primitive(PrimitiveType::I32) });
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
        ctx.current.push(AirInstruction::Alloca { target: result_slot, ty: Type::Primitive(PrimitiveType::I32) });
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
        ctx.current.push(AirInstruction::Load { target: loaded, ptr: result_slot, ty: Type::Primitive(PrimitiveType::I32) });
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
        ctx.current.push(AirInstruction::Alloca { target: result_slot, ty: Type::Primitive(PrimitiveType::I32) });
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
        ctx.current.push(AirInstruction::Load { target: loaded, ptr: result_slot, ty: Type::Primitive(PrimitiveType::I32) });
        AirValue::Register(loaded)
    }

    fn lower_match(&mut self, value: &HirExpr, arms: &[HirMatchArm],
                    ctx: &mut LoweringCtx, var_map: &mut HashMap<String, RegisterId>) -> AirValue {
        let match_val = self.lower_expr(value, ctx, var_map);
        let result_slot = self.fresh_reg();
        ctx.current.push(AirInstruction::Alloca { target: result_slot, ty: Type::Primitive(PrimitiveType::I32) });
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
                        ctx.current.push(AirInstruction::Alloca { target: ptr_reg, ty: Type::Primitive(PrimitiveType::I32) });
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
        ctx.current.push(AirInstruction::Load { target: loaded, ptr: result_slot, ty: Type::Primitive(PrimitiveType::I32) });
        AirValue::Register(loaded)
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
            _ => Type::Primitive(PrimitiveType::I32),
        },
        _ => Type::Primitive(PrimitiveType::I32),
    }
}
