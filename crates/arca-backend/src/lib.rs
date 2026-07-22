use arca_air::{AirFunction, AirInstruction, AirModule, AirTerminator, AirValue, BasicBlock, BlockId, RegisterId};
use arca_ast::BinaryOp;
use arca_typechecker::{PrimitiveType, Type};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendKind { Native, Llvm, C }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetArch { X86_64, Arm64, Riscv64, Wasm, C }

pub struct CodeGenerator {
    backend: BackendKind,
    target: TargetArch,
    var_names: HashMap<RegisterId, String>,
    var_types: HashMap<RegisterId, String>,
    copy_sources: HashMap<RegisterId, AirValue>,
    struct_inits: HashMap<RegisterId, (String, Vec<(String, AirValue)>)>,
    module_fns: HashMap<String, Type>,
    next_v: u32,
    next_block_label: u32,
    output: String,
    indent: usize,
    return_type_is_void: bool,
    return_type_is_string: bool,
}

impl CodeGenerator {
    pub fn new(backend: BackendKind, target: TargetArch) -> Self {
        Self {
            backend, target,
            var_names: HashMap::new(), var_types: HashMap::new(),
            copy_sources: HashMap::new(), struct_inits: HashMap::new(),
            module_fns: HashMap::new(),
            next_v: 0, next_block_label: 0,
            output: String::new(),             indent: 0, return_type_is_void: false, return_type_is_string: false,
        }
    }

    fn fresh_v(&mut self) -> String {
        let n = self.next_v; self.next_v += 1; format!("v{}", n)
    }

    fn fresh_block_label(&mut self) -> String {
        let n = self.next_block_label; self.next_block_label += 1; format!("bb_{}", n)
    }

    fn reg_name(&mut self, reg: RegisterId) -> String {
        if let Some(name) = self.var_names.get(&reg) {
            return name.clone();
        }
        let name = self.fresh_v();
        self.var_names.insert(reg, name.clone());
        name
    }

    fn resolve(&self, val: &AirValue) -> AirValue {
        match val {
            AirValue::Register(r) => {
                if let Some(src) = self.copy_sources.get(r) {
                    self.resolve(src)
                } else {
                    AirValue::Register(*r)
                }
            }
            _ => val.clone(),
        }
    }

    fn emit(&mut self, s: &str) { self.output.push_str(s); }
    fn emit_ln(&mut self, s: &str) {
        self.output.push_str(&"  ".repeat(self.indent));
        self.output.push_str(s); self.output.push('\n');
    }
    fn emit_indent(&mut self) { self.output.push_str(&"  ".repeat(self.indent)); }

    fn air_type_to_c<'a>(&self, ty: &'a Type) -> &'a str {
        match ty {
            Type::Primitive(p) => match p {
                PrimitiveType::I8 => "int8_t", PrimitiveType::I16 => "int16_t",
                PrimitiveType::I32 => "int32_t", PrimitiveType::I64 => "int64_t",
                PrimitiveType::U8 => "uint8_t", PrimitiveType::U16 => "uint16_t",
                PrimitiveType::U32 => "uint32_t", PrimitiveType::U64 => "uint64_t",
                PrimitiveType::F32 => "float", PrimitiveType::F64 => "double",
                PrimitiveType::Bool => "bool", PrimitiveType::String => "const char*",
                PrimitiveType::Char => "char", PrimitiveType::Void => "void",
            },
            Type::Struct { name, .. } => name.as_str(),
            Type::Reference { .. } => "void*",
            _ => "int64_t",
        }
    }

    fn type_for_instr(&self, instr: &AirInstruction) -> String {
        match instr {
            AirInstruction::Alloca { ty, .. } => self.air_type_to_c(ty).to_string(),
            AirInstruction::Load { .. } => "int64_t".to_string(),
            AirInstruction::Binary { op, left, right, .. } => match op {
                BinaryOp::Equal | BinaryOp::NotEqual | BinaryOp::Less | BinaryOp::LessEqual
                | BinaryOp::Greater | BinaryOp::GreaterEqual | BinaryOp::And | BinaryOp::Or => "bool".to_string(),
                BinaryOp::Add => {
                    if is_string_value(left, &self.var_types) || is_string_value(right, &self.var_types) {
                        "const char*".to_string()
                    } else {
                        "int64_t".to_string()
                    }
                }
                _ => "int64_t".to_string(),
            }
            AirInstruction::Call { fn_name, .. } => {
                // Check actual function return type from module first
                if let Some(ret_ty) = self.module_fns.get(fn_name) {
                    match ret_ty {
                        Type::Primitive(p) => match p {
                            PrimitiveType::Void => "void".to_string(),
                            PrimitiveType::String => "const char*".to_string(),
                            PrimitiveType::Bool => "bool".to_string(),
                            _ => "int64_t".to_string(),
                        },
                        _ => "int64_t".to_string(),
                    }
                } else if fn_name.starts_with("arca_std_") || fn_name == "arca_time_ns"
                    || fn_name == "serve" || fn_name == "arca_parse_int"
                    || fn_name == "arca_strcmp" || fn_name == "arca_starts_with"
                    || fn_name == "arca_str_rfind" || fn_name == "__arca_parse_int"
                    || fn_name == "__arca_starts_with" || fn_name == "__arca_str_rfind"
                    || fn_name == "__arca_str_contains" || fn_name == "__arca_ends_with"
                {
                    "int64_t".to_string()
                } else if fn_name.ends_with("_to_str") || fn_name.ends_with("user_json") || fn_name.ends_with("users_json")
                    || fn_name.starts_with("build_") || fn_name == "hash" || fn_name == "list_users"
                    || fn_name == "__arca_str_trim"
                    || fn_name == "env_get" || fn_name == "stdin_read_line"
                    || fn_name.starts_with("arca_path_")
                    || fn_name == "json_stringify"
                    || fn_name == "current_dir" || fn_name == "fs_metadata"
                    || fn_name == "path_normalize"
                {
                    "const char*".to_string()
                } else {
                    "int64_t".to_string()
                }
            }
            AirInstruction::StructInit { struct_name, .. } => {
                if struct_name.is_empty() { "int64_t".to_string() } else { struct_name.clone() }
            }
            AirInstruction::FieldLoad { object, field, .. } => {
                let resolved_obj = match self.resolve(&AirValue::Register(*object)) {
                    AirValue::Register(r) => r,
                    _ => *object,
                };
                if let Some((_, fields)) = self.struct_inits.get(&resolved_obj) {
                    if let Some((_, val)) = fields.iter().find(|(f, _)| f == field) {
                        match val {
                            AirValue::ConstString(_) => "const char*".to_string(),
                            AirValue::Register(r) => self.var_types.get(r).cloned().unwrap_or_else(|| "int64_t".to_string()),
                            _ => "int64_t".to_string(),
                        }
                    } else {
                        "int64_t".to_string()
                    }
                } else {
                    "int64_t".to_string()
                }
            }
            AirInstruction::Ref { .. } => "void*".to_string(),
            AirInstruction::Deref { .. } => "int64_t".to_string(),
            _ => "int64_t".to_string(),
        }
    }

    fn emit_air_value(&mut self, val: &AirValue) {
        let resolved = self.resolve(val);
        match &resolved {
            AirValue::ConstInt(n) => self.emit(&n.to_string()),
            AirValue::ConstFloat(f) => self.emit(&f.to_string()),
            AirValue::ConstBool(b) => self.emit(if *b { "1" } else { "0" }),
            AirValue::ConstString(s) => {
                let e = s.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n");
                self.emit(&format!("\"{}\"", e));
            }
            AirValue::Register(r) => {
                self.emit(&self.var_names.get(r).cloned().unwrap_or(format!("v{}", r.0)));
            }
        }
    }

    fn emit_air_value_str(&self, val: &AirValue) -> String {
        let resolved = self.resolve(val);
        match &resolved {
            AirValue::ConstInt(n) => n.to_string(),
            AirValue::ConstFloat(f) => f.to_string(),
            AirValue::ConstBool(b) => if *b { "1".to_string() } else { "0".to_string() },
            AirValue::ConstString(str) => {
                let e = str.replace('\\', "\\\\").replace('"', "\\\"").replace('\n', "\\n");
                format!("\"{}\"", e)
            }
            AirValue::Register(r) => {
                self.var_names.get(r).cloned().unwrap_or(format!("v{}", r.0))
            }
        }
    }

    pub fn generate_c_from_air(&mut self, module: &AirModule) -> String {
        self.output.clear();
        self.emit("// Generated by Arca C Backend (AIR)\n");
        self.emit("#include \"arca_runtime.h\"\n\n");

        // Store module function return types
        self.module_fns.clear();
        for (name, func) in &module.functions {
            self.module_fns.insert(name.clone(), func.return_type.clone());
        }

        // Collect struct definitions from all StructInit instructions
        let mut struct_defs: HashMap<String, Vec<(String, String)>> = HashMap::new();
        for func in module.functions.values() {
            for block in &func.blocks {
                for instr in &block.instructions {
                    if let AirInstruction::StructInit { struct_name, fields, .. } = instr {
                        if !struct_name.is_empty() && !struct_defs.contains_key(struct_name) {
                            let field_types: Vec<(String, String)> = fields.iter().map(|(n, v)| {
                                let fty = match v {
                                    AirValue::ConstString(_) => "const char*".to_string(),
                                    AirValue::ConstFloat(_) => "double".to_string(),
                                    AirValue::Register(r) => {
                                        self.var_types.get(r).cloned().unwrap_or_else(|| "int64_t".to_string())
                                    }
                                    _ => "int64_t".to_string(),
                                };
                                (n.clone(), fty)
                            }).collect();
                            struct_defs.insert(struct_name.clone(), field_types);
                        }
                    }
                }
            }
        }

        // Emit struct type definitions
        for (name, fields) in &struct_defs {
            self.emit(&format!("typedef struct {{ /* {} */ ", name));
            for (fn_, fty) in fields {
                self.emit(&format!("{} {}; ", fty, fn_));
            }
            self.emit(&format!("}} {};\n\n", name));
        }

        // Forward declarations for defined functions
        let mut defined_fns: HashSet<String> = module.functions.keys().cloned().collect();
        for (name, func) in &module.functions {
            let safe = name.replace('.', "_");
            if name == "main" { let _ = safe; }
            let safe_name = if name == "main" { "arca_main" } else { &safe };
            let ret = self.air_type_to_c(&func.return_type);
            let mut d = format!("{} {}(", ret, safe_name);
            if func.params.is_empty() { d.push_str("void"); }
            else {
                for (i, (_, pt)) in func.params.iter().enumerate() {
                    if i > 0 { d.push_str(", "); }
                    d.push_str(self.air_type_to_c(pt));
                }
            }
            d.push_str(");\n");
            self.emit(&d);
        }

        // Emit extern declarations for called-but-not-defined functions
        let mut extern_fns: HashSet<String> = HashSet::new();
        for func in module.functions.values() {
            for block in &func.blocks {
                for instr in &block.instructions {
                    if let AirInstruction::Call { fn_name, .. } = instr {
                        let safe = fn_name.replace('.', "_");
                        if !defined_fns.contains(fn_name) && !fn_name.starts_with("__arca_")
                            && !fn_name.starts_with("arca_")
                            && fn_name != "println" && fn_name != "print"
                            && fn_name != "sqrt" && fn_name != "sin" && fn_name != "cos" && fn_name != "abs"
                            && fn_name != "pow" && fn_name != "rand"
                            && fn_name != "sleep" && fn_name != "env_get"
                            && fn_name != "stdin_read_line"
                            && fn_name != "arca_fs_open" && fn_name != "arca_fs_close"
                            && fn_name != "arca_fs_exists" && fn_name != "arca_fs_remove"
                            && fn_name != "arca_path_extension" && fn_name != "arca_path_filename"
                            && fn_name != "arca_path_parent" && fn_name != "arca_path_join"
                            && fn_name != "fs_exists" && fn_name != "fs_remove"
                            && fn_name != "path_extension" && fn_name != "path_filename"
                            && fn_name != "path_parent" && fn_name != "path_join"
                            && fn_name != "exit" && fn_name != "arca_exit"
                            && fn_name != "json_stringify"
                            && fn_name != "env_set" && fn_name != "current_dir"
                            && fn_name != "stdout_write" && fn_name != "stderr_write"
                            && fn_name != "fs_rename" && fn_name != "fs_copy"
                            && fn_name != "fs_metadata" && fn_name != "path_normalize"
                        {
                            extern_fns.insert(safe);
                        }
                    }
                }
            }
        }
        for fn_name in &extern_fns {
            self.emit(&format!("int64_t {}();\n", fn_name));
        }
        self.emit("\n");

        for (name, func) in &module.functions {
            self.emit_function(name, func);
            self.emit("\n");
        }

        if module.functions.contains_key("main") {
            self.emit("int main(int argc, char** argv) {\n  arca_main();\n  return 0;\n}\n");
        }

        self.output.clone()
    }

    fn emit_function(&mut self, name: &str, func: &AirFunction) {
        self.var_names.clear(); self.var_types.clear();
        self.copy_sources.clear(); self.struct_inits.clear();
        self.next_v = 0; self.next_block_label = 0;
        self.return_type_is_void = matches!(&func.return_type, Type::Primitive(PrimitiveType::Void));
        self.return_type_is_string = matches!(&func.return_type, Type::Primitive(PrimitiveType::String));

        let safe_name = name.replace('.', "_");
        let safe_name = if name == "main" { "arca_main" } else { &safe_name };
        let ret_c = self.air_type_to_c(&func.return_type);

        // Build param register → C name mapping
        let mut param_reg_names: HashMap<RegisterId, String> = HashMap::new();
        for (i, (pname, _)) in func.params.iter().enumerate() {
            if i < func.param_registers.len() {
                param_reg_names.insert(func.param_registers[i], pname.replace('.', "_"));
                self.var_names.insert(func.param_registers[i], pname.replace('.', "_"));
            }
        }

        // Emit signature
        let mut sig = format!("{} {}(", ret_c, safe_name);
        if func.params.is_empty() { sig.push_str("void"); }
        else {
            for (i, (pname, pty)) in func.params.iter().enumerate() {
                if i > 0 { sig.push_str(", "); }
                sig.push_str(self.air_type_to_c(pty)); sig.push(' ');
                sig.push_str(&pname.replace('.', "_"));
            }
        }
        sig.push_str(") {\n");
        self.emit(&sig);
        self.indent += 1;

        // Pre-register param names and types
        for (reg, cname) in &param_reg_names {
            self.var_names.insert(*reg, cname.clone());
        }
        for (i, (_pname, pty)) in func.params.iter().enumerate() {
            if i < func.param_registers.len() {
                let reg = func.param_registers[i];
                let ct = self.air_type_to_c(pty).to_string();
                self.var_types.insert(reg, ct);
            }
        }

        // First pass: declare types with per-block copy propagation
        let mut reg_decls: Vec<(RegisterId, String)> = Vec::new();

        for block in &func.blocks {
            let mut block_stores: HashMap<RegisterId, AirValue> = HashMap::new();
            for instr in &block.instructions {
                // Track stores within this block for copy propagation
                if let AirInstruction::Store { ptr, val } = instr {
                    block_stores.insert(*ptr, val.clone());
                }
                // Record copy sources (same-block store-then-load) but still declare
                if let AirInstruction::Load { target, ptr, .. } = instr {
                    if let Some(src) = block_stores.get(ptr) {
                        self.copy_sources.insert(*target, src.clone());
                    }
                }
                let reg = match instr {
                    AirInstruction::Alloca { target, .. } => Some(*target),
                    AirInstruction::Load { target, .. } => {
                        if self.copy_sources.contains_key(target) { None } else { Some(*target) }
                    }
                    AirInstruction::Binary { target, .. } => Some(*target),
                    AirInstruction::Call { target, fn_name, .. } => {
                        // Skip declarations for void-returning calls
                        let is_void = fn_name == "println" || fn_name == "print" || fn_name.starts_with("show_")
                            || fn_name == "__arca_throw" || fn_name == "__arca_clear_last_error";
                        if is_void { None } else { *target }
                    }
                    AirInstruction::StructInit { target, struct_name, fields } => {
                        self.struct_inits.insert(*target, (struct_name.clone(), fields.clone()));
                        Some(*target)
                    }
                    AirInstruction::FieldLoad { target, .. } => Some(*target),
                    AirInstruction::Ref { target, .. } => Some(*target),
                    AirInstruction::Deref { target, .. } => Some(*target),
                    _ => None,
                };
                if let Some(r) = reg {
                    if !param_reg_names.contains_key(&r) && !reg_decls.iter().any(|(rr, _)| *rr == r) {
                        let ct = self.type_for_instr(instr).to_string();
                        reg_decls.push((r, ct));
                    }
                }
            }
        }

        // Emit declarations with proper types
        for (reg, ctype) in &reg_decls {
            let n = self.reg_name(*reg);
            self.emit_ln(&format!("{} {};", ctype, n));
            self.var_types.insert(*reg, ctype.clone());
        }

        // Propagate types for copy sources
        for (target, src) in &self.copy_sources {
            if let AirValue::Register(r) = src {
                if let Some(t) = self.var_types.get(r) {
                    self.var_types.insert(*target, t.clone());
                }
            }
        }

        // Emit blocks with per-function label numbering
        let mut block_labels: HashMap<BlockId, String> = HashMap::new();
        for b in &func.blocks {
            block_labels.insert(b.id, self.fresh_block_label());
        }

        let entry_label = block_labels.get(&func.entry_block).cloned().unwrap_or_default();
        let first_block = func.blocks.first().map(|b| b.id);
        let first_label = first_block.and_then(|id| block_labels.get(&id)).cloned();

        for block in &func.blocks {
            if let Some(label) = block_labels.get(&block.id) {
                if Some(label) != first_label.as_ref() && Some(label) != Some(&entry_label) {
                    self.emit_ln(&format!("{}: ;", label));
                }
            }
            for instr in &block.instructions {
                self.emit_air_instr(instr);
            }
            self.emit_air_terminator(&block.terminator, &block_labels);
        }

        self.indent -= 1;
        self.emit("}\n");
    }

    fn emit_air_instr(&mut self, instr: &AirInstruction) {
        match instr {
            AirInstruction::Alloca { .. } => {}
            AirInstruction::Store { ptr, val } => {
                let resolved = self.resolve(val);
                let pn = self.reg_name(*ptr);
                // Skip store if val is a StructInit or ptr is copy-propagated
                if let AirValue::Register(r) = &resolved {
                    if self.struct_inits.contains_key(r) {
                        return;
                    }
                }
                self.emit_indent();
                self.emit(&pn);
                self.emit(" = ");
                // Cast string/pointer values to int64_t since variables are int64_t slots
                let is_ptr = match &resolved {
                    AirValue::Register(r) => self.var_types.get(r).map(|t| t == "void*" || t == "const char*").unwrap_or(false),
                    _ => false,
                };
                if is_string_value(&resolved, &self.var_types) || is_ptr {
                    self.emit("(int64_t)");
                }
                self.emit_air_value(&resolved);
                self.emit(";\n");
            }
            AirInstruction::Load { target, ptr, .. } => {
                // Copy-propagated loads are skipped entirely
                if self.copy_sources.contains_key(target) { return; }
                let tn = self.reg_name(*target);
                let sn = self.reg_name(*ptr);
                self.emit_ln(&format!("{} = {};", tn, sn));
            }
            AirInstruction::Binary { target, op, left, right } => {
                let tn = self.reg_name(*target);
                let os = match op {
                    BinaryOp::Add => " + ", BinaryOp::Sub => " - ", BinaryOp::Mul => " * ",
                    BinaryOp::Div => " / ", BinaryOp::Rem => " % ",
                    BinaryOp::Equal => " == ", BinaryOp::NotEqual => " != ",
                    BinaryOp::Less => " < ", BinaryOp::LessEqual => " <= ",
                    BinaryOp::Greater => " > ", BinaryOp::GreaterEqual => " >= ",
                    BinaryOp::And => " && ", BinaryOp::Or => " || ",
                };
                // String-aware operations: use arca_strcmp for ==/!=, arca_strcat for string +
                if matches!(op, BinaryOp::Equal | BinaryOp::NotEqual) {
                    let is_not = matches!(op, BinaryOp::NotEqual);
                    let l_is_str = matches!(left, AirValue::ConstString(_)) ||
                        is_string_value(left, &self.var_types);
                    let r_is_str = matches!(right, AirValue::ConstString(_)) ||
                        is_string_value(right, &self.var_types);
                    if l_is_str || r_is_str {
                        self.emit_indent(); self.emit(&tn); self.emit(" = arca_strcmp((const char*)");
                        self.emit_air_value(left); self.emit(", (const char*)");
                        self.emit_air_value(right);
                        self.emit(&format!(") {} 0;\n", if is_not { "!=" } else { "==" }));
                    } else {
                        self.emit_indent(); self.emit(&tn); self.emit(" = ");
                        self.emit_air_value(left); self.emit(if is_not { " != " } else { " == " }); self.emit_air_value(right); self.emit(";\n");
                    }
                } else if matches!(op, BinaryOp::Add) {
                    let l_is_str = matches!(left, AirValue::ConstString(_)) ||
                        is_string_value(left, &self.var_types);
                    let r_is_str = matches!(right, AirValue::ConstString(_)) ||
                        is_string_value(right, &self.var_types);
                    if l_is_str || r_is_str {
                        self.emit_indent(); self.emit(&tn); self.emit(" = arca_strcat((const char*)");
                        self.emit_air_value(left); self.emit(", (const char*)");
                        self.emit_air_value(right); self.emit(");\n");
                        self.var_types.insert(*target, "const char*".to_string());
                    } else {
                        self.emit_indent(); self.emit(&tn); self.emit(" = ");
                        self.emit_air_value(left); self.emit(os); self.emit_air_value(right); self.emit(";\n");
                    }
                } else {
                    self.emit_indent(); self.emit(&tn); self.emit(" = ");
                    self.emit_air_value(left); self.emit(os); self.emit_air_value(right); self.emit(";\n");
                }
            }
            AirInstruction::Call { target, fn_name, args } => self.emit_air_call(target, fn_name, args),
            AirInstruction::StructInit { target, struct_name, fields } => {
                let tn = self.reg_name(*target);
                if struct_name.is_empty() {
                    // Anonymous struct: skip emission — used by FieldLoad instructions
                    // Just declare a zero-initialized placeholder
                    self.emit_ln(&format!("{} = 0;", tn));
                } else {
                    self.emit_indent(); self.emit(&format!("{} = ({}){{", tn, struct_name));
                    for (i, (fn_, fv)) in fields.iter().enumerate() {
                        if i > 0 { self.emit(", "); }
                        self.emit(&format!(".{}=", fn_)); self.emit_air_value(fv);
                    }
                    self.emit("};\n");
                }
            }
            AirInstruction::FieldLoad { target, object, field } => {
                let tn = self.reg_name(*target);
                let resolved_obj = match self.resolve(&AirValue::Register(*object)) {
                    AirValue::Register(r) => r,
                    _ => *object,
                };
                let on = self.reg_name(resolved_obj);
                let obj_type = self.var_types.get(&resolved_obj).cloned().unwrap_or_default();
                // If the object was created by a StructInit, use struct type for field access
                if let Some((sname, _)) = self.struct_inits.get(&resolved_obj) {
                    if !sname.is_empty() {
                        self.emit_ln(&format!("{} = (({}*)&{})->{};", tn, sname, on, field));
                    } else {
                        self.emit_ln(&format!("{} = *((int64_t*)((char*)&{} + offsetof_placeholder_{}));", tn, on, field));
                    }
                } else if !obj_type.is_empty() && obj_type != "int64_t" && obj_type != "const char*" && obj_type != "bool" {
                    self.emit_ln(&format!("{} = (int64_t){}.{};", tn, on, field));
                } else {
                    self.emit_ln(&format!("{} = (int64_t){}.{};", tn, on, field));
                }
            }
            AirInstruction::Ref { target, source } => {
                let tn = self.reg_name(*target);
                let sn = self.reg_name(*source);
                self.emit_ln(&format!("{} = &{};", tn, sn));
            }
            AirInstruction::Deref { target, ptr } => {
                let tn = self.reg_name(*target);
                let pn = self.reg_name(*ptr);
                self.emit_ln(&format!("{} = *{};", tn, pn));
            }
        }
    }

    fn emit_air_call(&mut self, target: &Option<RegisterId>, fn_name: &str, args: &[AirValue]) {
        match fn_name {
            "println" => {
                for arg in args {
                    let is_str = matches!(arg, AirValue::ConstString(_)) ||
                        if let AirValue::Register(r) = arg {
                            self.var_types.get(r).map(|t| t == "const char*").unwrap_or(false)
                        } else { false };
                    self.emit_indent(); self.emit(if is_str { "arca_print_string(" } else { "arca_print_int(" });
                    self.emit_air_value(arg); self.emit(");\n");
                }
                self.emit_ln("putchar('\\n');");
            }
            "print" => {
                for arg in args {
                    let is_str = matches!(arg, AirValue::ConstString(_)) ||
                        if let AirValue::Register(r) = arg {
                            self.var_types.get(r).map(|t| t == "const char*").unwrap_or(false)
                        } else { false };
                    self.emit_indent(); self.emit(if is_str { "arca_print_string(" } else { "arca_print_int(" });
                    self.emit_air_value(arg); self.emit(");\n");
                }
            }
            "Instant.now" | "Instant_now" | "now" => {
                if let Some(t) = target { let n = self.var_names.get(t).cloned().unwrap_or_default(); self.emit_ln(&format!("{} = arca_time_ns();", n)); }
            }
            "serve" => {
                let port = if args.len() == 1 {
                    match &args[0] {
                        AirValue::ConstInt(n) => n.to_string(),
                        AirValue::Register(r) => {
                            // Extract port from struct init
                            let mut p = "3000".to_string();
                            if let Some((_sname, fields)) = self.struct_inits.get(r) {
                                for (fn_, fv) in fields {
                                    if fn_ == "port" {
                                        p = self.emit_air_value_str(fv);
                                    }
                                }
                            }
                            p
                        }
                        _ => self.emit_air_value_str(&args[0]),
                    }
                } else { "3000".to_string() };
                let tn = target.and_then(|t| self.var_names.get(&t).cloned());
                if let Some(n) = tn { self.emit_ln(&format!("{} = arca_std_http_serve({});", n, port)); }
                else { self.emit_ln(&format!("arca_std_http_serve({});", port)); }
            }
            n if n.ends_with("elapsed_ms") => {
                if let Some(t) = target {
                    let tn = self.var_names.get(t).cloned().unwrap_or_default();
                    let tv = if !args.is_empty() { self.emit_air_value_str(&args[0]) } else { "0".to_string() };
                    self.emit_ln(&format!("{} = (arca_time_ns() - {}) / 1000000LL;", tn, tv));
                }
            }
            n if n.ends_with("elapsed_ns") => {
                if let Some(t) = target {
                    let tn = self.var_names.get(t).cloned().unwrap_or_default();
                    let tv = if !args.is_empty() { self.emit_air_value_str(&args[0]) } else { "0".to_string() };
                    self.emit_ln(&format!("{} = arca_time_ns() - {};", tn, tv));
                }
            }
            "__arca_parse_int" => {
                let tn = target.and_then(|t| self.var_names.get(&t).cloned()).unwrap_or_default();
                let s = if !args.is_empty() { self.emit_air_value_str(&args[0]) } else { "\"0\"".to_string() };
                self.emit_ln(&format!("{} = arca_parse_int((const char*){});", tn, s));
            }
            "__arca_to_string" => {
                let tn = target.and_then(|t| self.var_names.get(&t).cloned()).unwrap_or_default();
                let v = if !args.is_empty() { self.emit_air_value_str(&args[0]) } else { "0".to_string() };
                self.emit_ln(&format!("{} = arca_int_to_str({});", tn, v));
            }
            "__arca_int_to_str" => {
                let tn = target.and_then(|t| self.var_names.get(&t).cloned()).unwrap_or_default();
                let v = if !args.is_empty() { self.emit_air_value_str(&args[0]) } else { "0".to_string() };
                self.emit_ln(&format!("{} = arca_int_to_str({});", tn, v));
            }
            "__arca_starts_with" => {
                let tn = target.and_then(|t| self.var_names.get(&t).cloned()).unwrap_or_default();
                let s = if args.len() > 0 { self.emit_air_value_str(&args[0]) } else { "\"\"".to_string() };
                let p = if args.len() > 1 { self.emit_air_value_str(&args[1]) } else { "\"\"".to_string() };
                self.emit_ln(&format!("{} = arca_starts_with((const char*){}, (const char*){});", tn, s, p));
            }
            "__arca_str_rfind" => {
                let tn = target.and_then(|t| self.var_names.get(&t).cloned()).unwrap_or_default();
                let s = if args.len() > 0 { self.emit_air_value_str(&args[0]) } else { "\"\"".to_string() };
                // rfind takes a char — extract first char from string literal if needed
                let c = if args.len() > 1 {
                    let raw = self.emit_air_value_str(&args[1]);
                    // If it's a single-char string literal, use the char directly
                    if raw.len() > 2 && raw.starts_with('"') {
                        let ch = raw.as_bytes()[1] as char;
                        format!("'{}'", ch)
                    } else {
                        raw
                    }
                } else { "'\\0'".to_string() };
                self.emit_ln(&format!("{} = arca_str_rfind((const char*){}, {});", tn, s, c));
            }
            "__arca_str_slice" => {
                let tn = target.and_then(|t| self.var_names.get(&t).cloned()).unwrap_or_default();
                let s = if args.len() > 0 { self.emit_air_value_str(&args[0]) } else { "\"\"".to_string() };
                let start = if args.len() > 1 { self.emit_air_value_str(&args[1]) } else { "0".to_string() };
                self.emit_ln(&format!("{} = (int64_t)arca_str_slice((const char*){}, (int){});", tn, s, start));
            }
            "__arca_str_trim" => {
                let tn = target.and_then(|t| self.var_names.get(&t).cloned()).unwrap_or_default();
                let s = if !args.is_empty() { self.emit_air_value_str(&args[0]) } else { "\"\"".to_string() };
                self.emit_ln(&format!("{} = arca_str_trim((const char*){});", tn, s));
            }
            "__arca_str_contains" => {
                let tn = target.and_then(|t| self.var_names.get(&t).cloned()).unwrap_or_default();
                let s = if args.len() > 0 { self.emit_air_value_str(&args[0]) } else { "\"\"".to_string() };
                let sub = if args.len() > 1 { self.emit_air_value_str(&args[1]) } else { "\"\"".to_string() };
                self.emit_ln(&format!("{} = arca_str_contains((const char*){}, (const char*){});", tn, s, sub));
            }
            "__arca_ends_with" => {
                let tn = target.and_then(|t| self.var_names.get(&t).cloned()).unwrap_or_default();
                let s = if args.len() > 0 { self.emit_air_value_str(&args[0]) } else { "\"\"".to_string() };
                let suffix = if args.len() > 1 { self.emit_air_value_str(&args[1]) } else { "\"\"".to_string() };
                self.emit_ln(&format!("{} = arca_ends_with((const char*){}, (const char*){});", tn, s, suffix));
            }
            "__enum_tag" => {
                let tn = target.and_then(|t| self.var_names.get(&t).cloned()).unwrap_or_default();
                let tag = if !args.is_empty() { self.emit_air_value_str(&args[0]) } else { "0".to_string() };
                self.emit_ln(&format!("{} = {};", tn, tag));
            }
            // std/math
            "sqrt" | "sin" | "cos" | "abs" => {
                let tn = target.and_then(|t| self.var_names.get(&t).cloned()).unwrap_or_default();
                let x = if !args.is_empty() { self.emit_air_value_str(&args[0]) } else { "0".to_string() };
                let math_fn = if fn_name == "abs" { "llabs" } else { fn_name };
                self.emit_ln(&format!("{} = (int64_t){}((double){});", tn, math_fn, x));
            }
            "pow" => {
                let tn = target.and_then(|t| self.var_names.get(&t).cloned()).unwrap_or_default();
                let x = if args.len() > 0 { self.emit_air_value_str(&args[0]) } else { "0".to_string() };
                let y = if args.len() > 1 { self.emit_air_value_str(&args[1]) } else { "0".to_string() };
                self.emit_ln(&format!("{} = (int64_t)pow((double){}, (double){});", tn, x, y));
            }
            "rand" => {
                let tn = target.and_then(|t| self.var_names.get(&t).cloned()).unwrap_or_default();
                self.emit_ln(&format!("{} = (int64_t)rand();", tn));
            }
            "sleep" => {
                let ms = if !args.is_empty() { self.emit_air_value_str(&args[0]) } else { "0".to_string() };
                self.emit_ln(&format!("arca_sleep_ms({});", ms));
            }
            "env_get" => {
                let tn = target.and_then(|t| self.var_names.get(&t).cloned()).unwrap_or_default();
                let name = if !args.is_empty() { self.emit_air_value_str(&args[0]) } else { "\"\"".to_string() };
                self.emit_ln(&format!("{} = arca_env_get((const char*){});", tn, name));
            }
            "env_set" => {
                let name = if args.len() > 0 { self.emit_air_value_str(&args[0]) } else { "\"\"".to_string() };
                let val = if args.len() > 1 { self.emit_air_value_str(&args[1]) } else { "\"\"".to_string() };
                self.emit_ln(&format!("arca_env_set((const char*){}, (const char*){});", name, val));
            }
            "current_dir" => {
                let tn = target.and_then(|t| self.var_names.get(&t).cloned()).unwrap_or_default();
                self.emit_ln(&format!("{} = (int64_t)arca_current_dir();", tn));
            }
            "stdin_read_line" => {
                let tn = target.and_then(|t| self.var_names.get(&t).cloned()).unwrap_or_default();
                self.emit_ln(&format!("{} = (int64_t)arca_stdin_read_line();", tn));
            }
            "File.open" | "arca_fs_open" => {
                let tn = target.and_then(|t| self.var_names.get(&t).cloned()).unwrap_or_default();
                let path = if args.len() > 0 { self.emit_air_value_str(&args[0]) } else { "\"\"".to_string() };
                let mode = if args.len() > 1 { self.emit_air_value_str(&args[1]) } else { "\"r\"".to_string() };
                self.emit_ln(&format!("{} = arca_fs_open((const char*){}, (const char*){});", tn, path, mode));
            }
            "fs_exists" => {
                let tn = target.and_then(|t| self.var_names.get(&t).cloned()).unwrap_or_default();
                let path = if !args.is_empty() { self.emit_air_value_str(&args[0]) } else { "\"\"".to_string() };
                self.emit_ln(&format!("{} = arca_fs_exists((const char*){});", tn, path));
            }
            "fs_remove" => {
                let tn = target.and_then(|t| self.var_names.get(&t).cloned()).unwrap_or_default();
                let path = if !args.is_empty() { self.emit_air_value_str(&args[0]) } else { "\"\"".to_string() };
                self.emit_ln(&format!("{} = arca_fs_remove((const char*){});", tn, path));
            }
            "fs_rename" => {
                let tn = target.and_then(|t| self.var_names.get(&t).cloned()).unwrap_or_default();
                let old = if args.len() > 0 { self.emit_air_value_str(&args[0]) } else { "\"\"".to_string() };
                let new_ = if args.len() > 1 { self.emit_air_value_str(&args[1]) } else { "\"\"".to_string() };
                self.emit_ln(&format!("{} = arca_fs_rename((const char*){}, (const char*){});", tn, old, new_));
            }
            "fs_copy" => {
                let tn = target.and_then(|t| self.var_names.get(&t).cloned()).unwrap_or_default();
                let src = if args.len() > 0 { self.emit_air_value_str(&args[0]) } else { "\"\"".to_string() };
                let dst = if args.len() > 1 { self.emit_air_value_str(&args[1]) } else { "\"\"".to_string() };
                self.emit_ln(&format!("{} = arca_fs_copy((const char*){}, (const char*){});", tn, src, dst));
            }
            "fs_metadata" => {
                let tn = target.and_then(|t| self.var_names.get(&t).cloned()).unwrap_or_default();
                let path = if !args.is_empty() { self.emit_air_value_str(&args[0]) } else { "\"\"".to_string() };
                self.emit_ln(&format!("{} = (int64_t)arca_fs_metadata((const char*){});", tn, path));
            }
            "path_extension" | "path_filename" | "path_parent" => {
                let tn = target.and_then(|t| self.var_names.get(&t).cloned()).unwrap_or_default();
                let path = if !args.is_empty() { self.emit_air_value_str(&args[0]) } else { "\"\"".to_string() };
                self.emit_ln(&format!("{} = arca_path_{}((const char*){});", tn, fn_name, path));
            }
            "path_join" => {
                let tn = target.and_then(|t| self.var_names.get(&t).cloned()).unwrap_or_default();
                let a = if args.len() > 0 { self.emit_air_value_str(&args[0]) } else { "\"\"".to_string() };
                let b = if args.len() > 1 { self.emit_air_value_str(&args[1]) } else { "\"\"".to_string() };
                self.emit_ln(&format!("{} = arca_path_join((const char*){}, (const char*){});", tn, a, b));
            }
            "path_normalize" => {
                let tn = target.and_then(|t| self.var_names.get(&t).cloned()).unwrap_or_default();
                let path = if !args.is_empty() { self.emit_air_value_str(&args[0]) } else { "\"\"".to_string() };
                self.emit_ln(&format!("{} = arca_path_normalize((const char*){});", tn, path));
            }
            "stdout_write" => {
                let s = if !args.is_empty() { self.emit_air_value_str(&args[0]) } else { "\"\"".to_string() };
                self.emit_ln(&format!("arca_stdout_write((const char*){});", s));
            }
            "stderr_write" => {
                let s = if !args.is_empty() { self.emit_air_value_str(&args[0]) } else { "\"\"".to_string() };
                self.emit_ln(&format!("arca_stderr_write((const char*){});", s));
            }
            "exit" => {
                let code = if !args.is_empty() { self.emit_air_value_str(&args[0]) } else { "0".to_string() };
                self.emit_ln(&format!("arca_exit({});", code));
            }
            "json_stringify" => {
                let tn = target.and_then(|t| self.var_names.get(&t).cloned()).unwrap_or_default();
                let s = if !args.is_empty() { self.emit_air_value_str(&args[0]) } else { "\"\"".to_string() };
                self.emit_ln(&format!("{} = (int64_t)arca_json_stringify((const char*){});", tn, s));
            }
            "parse" | "arca_json_parse" => {
                let tn = target.and_then(|t| self.var_names.get(&t).cloned()).unwrap_or_default();
                let s = if args.len() > 0 { self.emit_air_value_str(&args[0]) } else { "\"\"".to_string() };
                let k = if args.len() > 1 { self.emit_air_value_str(&args[1]) } else { "\"\"".to_string() };
                self.emit_ln(&format!("{} = (int64_t)arca_json_parse((const char*){}, (const char*){});", tn, s, k));
            }
            "split" | "arca_str_split" => {
                let tn = target.and_then(|t| self.var_names.get(&t).cloned()).unwrap_or_default();
                let s = if args.len() > 0 { self.emit_air_value_str(&args[0]) } else { "\"\"".to_string() };
                let d = if args.len() > 1 { self.emit_air_value_str(&args[1]) } else { "\"\"".to_string() };
                let idx = if args.len() > 2 { self.emit_air_value_str(&args[2]) } else { "0".to_string() };
                self.emit_ln(&format!("{} = (int64_t)arca_str_split((const char*){}, (const char*){}, (int){});", tn, s, d, idx));
            }
            "replace" | "arca_str_replace" => {
                let tn = target.and_then(|t| self.var_names.get(&t).cloned()).unwrap_or_default();
                let s = if args.len() > 0 { self.emit_air_value_str(&args[0]) } else { "\"\"".to_string() };
                let f = if args.len() > 1 { self.emit_air_value_str(&args[1]) } else { "\"\"".to_string() };
                let t_val = if args.len() > 2 { self.emit_air_value_str(&args[2]) } else { "\"\"".to_string() };
                self.emit_ln(&format!("{} = (int64_t)arca_str_replace((const char*){}, (const char*){}, (const char*){});", tn, s, f, t_val));
            }
            "format" | "arca_str_format" => {
                let tn = target.and_then(|t| self.var_names.get(&t).cloned()).unwrap_or_default();
                let fmt = if args.len() > 0 { self.emit_air_value_str(&args[0]) } else { "\"\"".to_string() };
                let arg = if args.len() > 1 { self.emit_air_value_str(&args[1]) } else { "\"\"".to_string() };
                self.emit_ln(&format!("{} = (int64_t)arca_str_format((const char*){}, (const char*){});", tn, fmt, arg));
            }
            "arca_fs_mkdir" => {
                let tn = target.and_then(|t| self.var_names.get(&t).cloned()).unwrap_or_default();
                let path = if !args.is_empty() { self.emit_air_value_str(&args[0]) } else { "\"\"".to_string() };
                self.emit_ln(&format!("{} = arca_fs_mkdir((const char*){});", tn, path));
            }
            "arca_fs_rmdir" => {
                let tn = target.and_then(|t| self.var_names.get(&t).cloned()).unwrap_or_default();
                let path = if !args.is_empty() { self.emit_air_value_str(&args[0]) } else { "\"\"".to_string() };
                self.emit_ln(&format!("{} = arca_fs_rmdir((const char*){});", tn, path));
            }
            "arca_fs_read_dir" => {
                let tn = target.and_then(|t| self.var_names.get(&t).cloned()).unwrap_or_default();
                let path = if !args.is_empty() { self.emit_air_value_str(&args[0]) } else { "\"\"".to_string() };
                self.emit_ln(&format!("{} = (int64_t)arca_fs_read_dir((const char*){});", tn, path));
            }
            "spawn" | "arca_process_spawn" => {
                let tn = target.and_then(|t| self.var_names.get(&t).cloned()).unwrap_or_default();
                let cmd = if !args.is_empty() { self.emit_air_value_str(&args[0]) } else { "\"\"".to_string() };
                self.emit_ln(&format!("{} = arca_process_spawn((const char*){});", tn, cmd));
            }
            "wait" | "arca_process_wait" => {
                let tn = target.and_then(|t| self.var_names.get(&t).cloned()).unwrap_or_default();
                let pid = if !args.is_empty() { self.emit_air_value_str(&args[0]) } else { "0".to_string() };
                self.emit_ln(&format!("{} = arca_process_wait((int64_t){});", tn, pid));
            }
            "arca_process_command" => {
                let tn = target.and_then(|t| self.var_names.get(&t).cloned()).unwrap_or_default();
                let cmd = if !args.is_empty() { self.emit_air_value_str(&args[0]) } else { "\"\"".to_string() };
                self.emit_ln(&format!("{} = arca_process_command((const char*){});", tn, cmd));
            }
            n if n.starts_with("arca_vec_") || n.starts_with("arca_map_") || n.starts_with("arca_set_")
                || n.starts_with("arca_queue_") || n.starts_with("arca_deque_")
                || n.starts_with("arca_heap_") || n.starts_with("arca_list_") => {
                let pre = target.and_then(|t| self.var_names.get(&t).cloned()).map(|n| format!("{} = ", n)).unwrap_or_default();
                self.emit_indent(); self.emit(&pre); self.emit(fn_name); self.emit("(");
                for (i, arg) in args.iter().enumerate() { if i > 0 { self.emit(", "); } self.emit_air_value(arg); }
                self.emit(");\n");
            }
            "arca_scheduler_spawn" | "__arca_spawn" => {
                let fn_arg = if !args.is_empty() {
                    match &args[0] {
                        AirValue::ConstString(s) => s.clone(),
                        other => self.emit_air_value_str(other),
                    }
                } else { "NULL".to_string() };
                let data_arg = if args.len() > 1 { self.emit_air_value_str(&args[1]) } else { "0".to_string() };
                self.emit_ln(&format!("arca_scheduler_spawn((void(*)(void*)){}, (void*)(intptr_t){});", fn_arg, data_arg));
            }
            "arca_channel_create" => {
                let tn = target.and_then(|t| self.var_names.get(&t).cloned()).unwrap_or_default();
                let cap = if !args.is_empty() { self.emit_air_value_str(&args[0]) } else { "16".to_string() };
                if !tn.is_empty() {
                    self.emit_ln(&format!("{} = (int64_t)arca_channel_create({});", tn, cap));
                } else {
                    self.emit_ln(&format!("arca_channel_create({});", cap));
                }
            }
            "arca_channel_send" => {
                let chan = if !args.is_empty() { self.emit_air_value_str(&args[0]) } else { "0".to_string() };
                let val = if args.len() > 1 { self.emit_air_value_str(&args[1]) } else { "0".to_string() };
                self.emit_ln(&format!("arca_channel_send((void*){}, (int64_t){});", chan, val));
            }
            "arca_channel_recv" => {
                let tn = target.and_then(|t| self.var_names.get(&t).cloned()).unwrap_or_default();
                let chan = if !args.is_empty() { self.emit_air_value_str(&args[0]) } else { "0".to_string() };
                if !tn.is_empty() {
                    self.emit_ln(&format!("{} = arca_channel_recv((void*){});", tn, chan));
                } else {
                    self.emit_ln(&format!("arca_channel_recv((void*){});", chan));
                }
            }
            "Response.ok" | "Response.text" | "Response.html" | "Response.json"
            | "Response.not_found" | "Response.bad_request" | "Response.internal_error"
            | "Response.redirect" => {
                // Response factory methods — return 0 (runtime handles default)
                if let Some(t) = target {
                    let tn = self.var_names.get(t).cloned().unwrap_or_default();
                    self.emit_ln(&format!("{} = 0;", tn));
                }
            }
            _ => {
                let safe_name = fn_name.replace('.', "_");
                let pre = target.and_then(|t| self.var_names.get(&t).cloned()).map(|n| format!("{} = ", n)).unwrap_or_default();
                self.emit_indent(); self.emit(&pre); self.emit(&safe_name); self.emit("(");
                for (i, arg) in args.iter().enumerate() { if i > 0 { self.emit(", "); } self.emit_air_value(arg); }
                self.emit(");\n");
            }
        }
    }

    fn emit_air_terminator(&mut self, term: &AirTerminator, label_map: &HashMap<BlockId, String>) {
        match term {
            AirTerminator::Br(t) => {
                let lbl = label_map.get(t).cloned().unwrap_or(format!("bb_{}", t.0));
                self.emit_ln(&format!("goto {};", lbl));
            }
            AirTerminator::CondBr { cond, then_block, else_block } => {
                let tl = label_map.get(then_block).cloned().unwrap_or(format!("bb_{}", then_block.0));
                let el = label_map.get(else_block).cloned().unwrap_or(format!("bb_{}", else_block.0));
                self.emit_indent(); self.emit("if ("); self.emit_air_value(cond);
                self.emit(&format!(") goto {}; else goto {};\n", tl, el));
            }
            AirTerminator::Ret(opt) => {
                match opt {
                    Some(val) if !self.return_type_is_void => {
                        self.emit_indent();
                        if self.return_type_is_string {
                            self.emit("return (const char*)");
                        } else {
                            self.emit("return ");
                        }
                        self.emit_air_value(val); self.emit(";\n");
                    }
                    _ => self.emit_ln("return;"),
                }
            }
            AirTerminator::Unreachable => self.emit_ln("__builtin_unreachable();"),
        }
    }
}

/// Check if an AirValue represents a string at runtime
fn is_string_value(val: &AirValue, var_types: &HashMap<RegisterId, String>) -> bool {
    match val {
        AirValue::ConstString(_) => true,
        AirValue::Register(r) => var_types.get(r).map(|t| t == "const char*").unwrap_or(false),
        _ => false,
    }
}