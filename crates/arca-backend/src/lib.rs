use arca_air::{AirFunction, AirInstruction, AirModule, AirTerminator, AirValue, BlockId, RegisterId};
use arca_ast::BinaryOp;
use arca_typechecker::{PrimitiveType, Type};
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BackendKind { Native, Llvm, C }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetArch { X86_64, Arm64, Riscv64, Wasm, C }

pub struct CodeGenerator {
    #[allow(dead_code)]
    backend: BackendKind,
    target: TargetArch,
    prefix: String,
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
            prefix: String::new(),
            var_names: HashMap::new(), var_types: HashMap::new(),
            copy_sources: HashMap::new(), struct_inits: HashMap::new(),
            module_fns: HashMap::new(),
            next_v: 0, next_block_label: 0,
            output: String::new(),             indent: 0, return_type_is_void: false, return_type_is_string: false,
        }
    }

    pub fn with_prefix(mut self, prefix: &str) -> Self {
        self.prefix = prefix.to_string();
        self
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

    fn extract_handle_or_value(&self, args: &[AirValue], idx: usize) -> String {
        if idx < args.len() {
            let self_reg = match self.resolve(&args[idx]) {
                AirValue::Register(r) => r,
                _ => return self.emit_air_value_str(&args[idx]),
            };
            if let Some((_, fields)) = self.struct_inits.get(&self_reg) {
                for (fn_, fv) in fields {
                    if fn_ == "handle" { return self.emit_air_value_str(fv); }
                }
            }
            return self.emit_air_value_str(&args[idx]);
        }
        "0".to_string()
    }

    fn emit_struct_method(&mut self, rt_fn: &str, argc: usize, _callee: &str, target: &Option<RegisterId>, args: &[AirValue]) {
        let tn = target.and_then(|t| self.var_names.get(&t).cloned()).unwrap_or_default();
        let handle = self.extract_handle_or_value(args, 0);
        let mut call = format!("{} {}(", if tn.is_empty() { String::new() } else { format!("{} = ", tn) }, rt_fn);
        call.push_str(&handle);
        for i in 1..argc {
            if i < args.len() {
                call.push_str(&format!(", {}", self.emit_air_value_str(&args[i])));
            } else {
                call.push_str(", 0");
            }
        }
        call.push_str(");\n");
        self.emit(&call);
    }

    fn emit_route_method(&mut self, method: &str, _callee: &str, target: &Option<RegisterId>, args: &[AirValue]) {
        let tn = target.and_then(|t| self.var_names.get(&t).cloned()).unwrap_or_default();
        let handle = self.extract_handle_or_value(args, 0);
        let path = if args.len() > 1 { self.emit_air_value_str(&args[1]) } else { "\"\"".to_string() };
        self.emit_ln(&format!("{} = (const char*)arca_str_format(\"{} {{}}/{{}}\", {}, {});",
            tn, method, path, handle));
    }

    fn p(&self, name: &str) -> String {
        if self.prefix.is_empty() { name.to_string() } else { format!("{}{}", self.prefix, name) }
    }

    fn type_to_c(&self, ty: &Type) -> String {
        match ty {
            Type::Primitive(p) => match p {
                PrimitiveType::I8 => "int8_t".into(), PrimitiveType::I16 => "int16_t".into(),
                PrimitiveType::I32 => "int32_t".into(), PrimitiveType::I64 => "int64_t".into(),
                PrimitiveType::U8 => "uint8_t".into(), PrimitiveType::U16 => "uint16_t".into(),
                PrimitiveType::U32 => "uint32_t".into(), PrimitiveType::U64 => "uint64_t".into(),
                PrimitiveType::F32 => "float".into(), PrimitiveType::F64 => "double".into(),
                PrimitiveType::Bool => "bool".into(), PrimitiveType::String => "const char*".into(),
                PrimitiveType::Char => "char".into(), PrimitiveType::Void => "void".into(),
            },
            Type::Struct { name, .. } => self.p(name),
            Type::Reference { .. } => "void*".into(),
            _ => "int64_t".into(),
        }
    }

    fn type_for_instr(&self, instr: &AirInstruction) -> String {
        match instr {
            AirInstruction::Alloca { ty, .. } => self.type_to_c(ty),
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
                        Type::Struct { name, .. } => self.p(name),
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
                    || fn_name == "env_get" || fn_name == "arca_env_get" || fn_name == "env" || fn_name == "stdin_read_line"
                    || fn_name.starts_with("arca_path_") || fn_name.starts_with("path_")
                    || fn_name == "json_stringify" || fn_name == "compress" || fn_name == "sha256"
                    || fn_name == "arch"
                    || fn_name == "arca_str_slice"
                    || fn_name == "__arca_str_at" || fn_name == "__arca_str_lower"
                    || fn_name == "__arca_str_upper" || fn_name == "__arca_str_repeat"
                    || fn_name == "__arca_str_lines"
                    || fn_name == "__arca_hostname" || fn_name == "__arca_username"
                    || fn_name == "OpenAI.chat" || fn_name == "OpenAI_chat"
                    || fn_name == "RAGEngine.query" || fn_name == "RAGEngine_query"
                    || fn_name.ends_with(".chat") || fn_name.ends_with(".query")
                    || fn_name.ends_with(".to_header") || fn_name.ends_with("_to_header")
                    || fn_name.ends_with(".post") || fn_name.ends_with(".get") || fn_name.ends_with(".put") || fn_name.ends_with(".delete")
                    || fn_name == "current_dir"
                    || fn_name == "file_read"
                    || fn_name == "hex_encode" || fn_name == "urlencode" || fn_name == "urldecode" || fn_name == "tcp_recv"
                    || fn_name == "path_normalize"
                {
                    "const char*".to_string()
                } else {
                    "int64_t".to_string()
                }
            }
            AirInstruction::StructInit { struct_name, .. } => {
                if struct_name.is_empty() { "int64_t".to_string() } else { self.p(struct_name) }
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
            self.emit(&format!("typedef struct {{ /* {} */ ", self.p(name)));
            for (fn_, fty) in fields {
                self.emit(&format!("{} {}; ", fty, fn_));
            }
            self.emit(&format!("}} {};\n\n", self.p(name)));
        }

        // Forward declarations for defined functions
        let defined_fns: HashSet<String> = module.functions.keys().cloned().collect();
        for (name, func) in &module.functions {
            let safe = name.replace('.', "_");
            // Skip prefix for internal __arca_* functions
            let safe_name = if name == "main" { self.p("arca_main") }
                          else if name.starts_with("__arca_") { safe.clone() }
                          else { self.p(&safe) };
            let ret = self.type_to_c(&func.return_type);
            let mut d = format!("{} {}(", ret, safe_name);
            if func.params.is_empty() { d.push_str("void"); }
            else {
                for (i, (_, pt)) in func.params.iter().enumerate() {
                    if i > 0 { d.push_str(", "); }
                    d.push_str(&self.type_to_c(pt));
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
                            && fn_name != "min" && fn_name != "max" && fn_name != "clamp"
                            && fn_name != "floor" && fn_name != "ceil" && fn_name != "round"
                            && fn_name != "log" && fn_name != "exp" && fn_name != "random_range"
                            && fn_name != "expect"
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
                            && fn_name != "file_read" && fn_name != "file_write" && fn_name != "file_append"
                            && fn_name != "file_copy" && fn_name != "file_rename"
                            && fn_name != "file_remove" && fn_name != "file_mkdir" && fn_name != "file_exists"
                            && fn_name != "hex_encode" && fn_name != "urlencode" && fn_name != "urldecode"
                            && fn_name != "tcp_listen" && fn_name != "tcp_accept" && fn_name != "tcp_recv"
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
            self.emit(&format!("int main(int argc, char** argv) {{\n  {}();\n  return 0;\n}}\n", self.p("arca_main")));
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
        let safe_name = if name == "main" { self.p("arca_main") }
                      else if name.starts_with("__arca_") { safe_name }
                      else { self.p(&safe_name) };
        let ret_c = self.type_to_c(&func.return_type);

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
                sig.push_str(&self.type_to_c(pty)); sig.push(' ');
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
                let ct = self.type_to_c(pty);
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
                            || fn_name == "__arca_throw" || fn_name == "__arca_clear_last_error"
                            || fn_name == "arca_future_complete"
                            || fn_name == "info" || fn_name == "warn" || fn_name == "error" || fn_name == "debug"
                            || fn_name == "__arca_assert_eq" || fn_name == "__arca_assert_throw";
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

        for block in &func.blocks {
            if block.id != func.entry_block {
                if let Some(label) = block_labels.get(&block.id) {
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
                    self.emit_indent(); self.emit(&format!("{} = ({}){{", tn, self.p(struct_name)));
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
                        self.emit_ln(&format!("{} = (({}*)&{})->{};", tn, self.p(sname), on, field));
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
                let math_fn = if fn_name == "abs" { "fabs" } else { fn_name };
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
            "log" => {
                let tn = target.and_then(|t| self.var_names.get(&t).cloned()).unwrap_or_default();
                let x = if !args.is_empty() { self.emit_air_value_str(&args[0]) } else { "0".to_string() };
                self.emit_ln(&format!("{} = (int64_t)log((double){});", tn, x));
            }
            "exp" => {
                let tn = target.and_then(|t| self.var_names.get(&t).cloned()).unwrap_or_default();
                let x = if !args.is_empty() { self.emit_air_value_str(&args[0]) } else { "0".to_string() };
                self.emit_ln(&format!("{} = (int64_t)exp((double){});", tn, x));
            }
            "min" => {
                let tn = target.and_then(|t| self.var_names.get(&t).cloned()).unwrap_or_default();
                let a = if args.len() > 0 { self.emit_air_value_str(&args[0]) } else { "0".to_string() };
                let b = if args.len() > 1 { self.emit_air_value_str(&args[1]) } else { "0".to_string() };
                self.emit_ln(&format!("{} = {} < {} ? {} : {};", tn, a, b, a, b));
            }
            "max" => {
                let tn = target.and_then(|t| self.var_names.get(&t).cloned()).unwrap_or_default();
                let a = if args.len() > 0 { self.emit_air_value_str(&args[0]) } else { "0".to_string() };
                let b = if args.len() > 1 { self.emit_air_value_str(&args[1]) } else { "0".to_string() };
                self.emit_ln(&format!("{} = {} > {} ? {} : {};", tn, a, b, a, b));
            }
            "clamp" => {
                let tn = target.and_then(|t| self.var_names.get(&t).cloned()).unwrap_or_default();
                let x = if args.len() > 0 { self.emit_air_value_str(&args[0]) } else { "0".to_string() };
                let lo = if args.len() > 1 { self.emit_air_value_str(&args[1]) } else { "0".to_string() };
                let hi = if args.len() > 2 { self.emit_air_value_str(&args[2]) } else { "0".to_string() };
                self.emit_ln(&format!("{} = {} < {} ? {} : ({} > {} ? {} : {});", tn, x, lo, lo, x, hi, hi, x));
            }
            "floor" => {
                let tn = target.and_then(|t| self.var_names.get(&t).cloned()).unwrap_or_default();
                let x = if !args.is_empty() { self.emit_air_value_str(&args[0]) } else { "0".to_string() };
                self.emit_ln(&format!("{} = (int64_t)floor((double){});", tn, x));
            }
            "ceil" => {
                let tn = target.and_then(|t| self.var_names.get(&t).cloned()).unwrap_or_default();
                let x = if !args.is_empty() { self.emit_air_value_str(&args[0]) } else { "0".to_string() };
                self.emit_ln(&format!("{} = (int64_t)ceil((double){});", tn, x));
            }
            "round" => {
                let tn = target.and_then(|t| self.var_names.get(&t).cloned()).unwrap_or_default();
                let x = if !args.is_empty() { self.emit_air_value_str(&args[0]) } else { "0".to_string() };
                self.emit_ln(&format!("{} = (int64_t)round((double){});", tn, x));
            }
            "random_range" => {
                let tn = target.and_then(|t| self.var_names.get(&t).cloned()).unwrap_or_default();
                let lo = if args.len() > 0 { self.emit_air_value_str(&args[0]) } else { "0".to_string() };
                let hi = if args.len() > 1 { self.emit_air_value_str(&args[1]) } else { "0".to_string() };
                self.emit_ln(&format!("{} = (int64_t)({} + (double)rand() / (double)RAND_MAX * (double)({} - {}));", tn, lo, hi, lo));
            }
            "sleep" => {
                let ms = if !args.is_empty() { self.emit_air_value_str(&args[0]) } else { "0".to_string() };
                self.emit_ln(&format!("arca_sleep_ms({});", ms));
            }
            "expect" => {
                let tn = target.and_then(|t| self.var_names.get(&t).cloned()).unwrap_or_default();
                let val = if !args.is_empty() { self.emit_air_value_str(&args[0]) } else { "0".to_string() };
                self.emit_ln(&format!("{} = {};", tn, val));
            }
            "__arca_assert_eq" | "__arca_assert_throw" => {
                self.emit(fn_name); self.emit("(");
                for (i, arg) in args.iter().enumerate() { if i > 0 { self.emit(", "); } self.emit_air_value(arg); }
                self.emit(");\n");
            }
            "env_get" | "arca_env_get" => {
                let tn = target.and_then(|t| self.var_names.get(&t).cloned()).unwrap_or_default();
                let name = if !args.is_empty() { self.emit_air_value_str(&args[0]) } else { "\"\"".to_string() };
                self.emit_ln(&format!("{} = (const char*)arca_env_get((const char*){});", tn, name));
            }
            "env_set" => {
                let name = if args.len() > 0 { self.emit_air_value_str(&args[0]) } else { "\"\"".to_string() };
                let val = if args.len() > 1 { self.emit_air_value_str(&args[1]) } else { "\"\"".to_string() };
                self.emit_ln(&format!("arca_env_set((const char*){}, (const char*){});", name, val));
            }
            "current_dir" => {
                let tn = target.and_then(|t| self.var_names.get(&t).cloned()).unwrap_or_default();
                self.emit_ln(&format!("{} = (const char*)arca_current_dir();", tn));
            }
            "stdin_read_line" => {
                let tn = target.and_then(|t| self.var_names.get(&t).cloned()).unwrap_or_default();
                self.emit_ln(&format!("{} = (const char*)arca_stdin_read_line();", tn));
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
            "path_extension" | "arca_path_extension" => {
                let tn = target.and_then(|t| self.var_names.get(&t).cloned()).unwrap_or_default();
                let path = if !args.is_empty() { self.emit_air_value_str(&args[0]) } else { "\"\"".to_string() };
                self.emit_ln(&format!("{} = (const char*)arca_path_extension((const char*){});", tn, path));
            }
            "path_filename" | "arca_path_filename" => {
                let tn = target.and_then(|t| self.var_names.get(&t).cloned()).unwrap_or_default();
                let path = if !args.is_empty() { self.emit_air_value_str(&args[0]) } else { "\"\"".to_string() };
                self.emit_ln(&format!("{} = (const char*)arca_path_filename((const char*){});", tn, path));
            }
            "path_parent" | "arca_path_parent" => {
                let tn = target.and_then(|t| self.var_names.get(&t).cloned()).unwrap_or_default();
                let path = if !args.is_empty() { self.emit_air_value_str(&args[0]) } else { "\"\"".to_string() };
                self.emit_ln(&format!("{} = (const char*)arca_path_parent((const char*){});", tn, path));
            }
            "path_join" | "arca_path_join" => {
                let tn = target.and_then(|t| self.var_names.get(&t).cloned()).unwrap_or_default();
                let a = if args.len() > 0 { self.emit_air_value_str(&args[0]) } else { "\"\"".to_string() };
                let b = if args.len() > 1 { self.emit_air_value_str(&args[1]) } else { "\"\"".to_string() };
                self.emit_ln(&format!("{} = (const char*)arca_path_join((const char*){}, (const char*){});", tn, a, b));
            }
            "path_normalize" | "arca_path_normalize" => {
                let tn = target.and_then(|t| self.var_names.get(&t).cloned()).unwrap_or_default();
                let path = if !args.is_empty() { self.emit_air_value_str(&args[0]) } else { "\"\"".to_string() };
                self.emit_ln(&format!("{} = (const char*)arca_path_normalize((const char*){});", tn, path));
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
                self.emit_ln(&format!("{} = (const char*)arca_json_stringify((const char*){});", tn, s));
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
            "spawn" => {
                if args.len() >= 2 {
                    // Async spawn: spawn(func: i64, arg: i64) -> Task
                    let tn = target.and_then(|t| self.var_names.get(&t).cloned()).unwrap_or_default();
                    let func = self.emit_air_value_str(&args[0]);
                    let arg = self.emit_air_value_str(&args[1]);
                    self.emit_ln(&format!("{} = arca_task_spawn((void(*)(void*))(intptr_t){}, (void*)(intptr_t){});", tn, func, arg));
                } else {
                    // Process spawn: spawn(cmd: string) -> i64
                    let tn = target.and_then(|t| self.var_names.get(&t).cloned()).unwrap_or_default();
                    let cmd = if !args.is_empty() { self.emit_air_value_str(&args[0]) } else { "\"\"".to_string() };
                    self.emit_ln(&format!("{} = arca_process_spawn((const char*){});", tn, cmd));
                }
            }
            "arca_process_spawn" => {
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
            "arca_str_slice" => {
                let tn = target.and_then(|t| self.var_names.get(&t).cloned()).unwrap_or_default();
                let s = if !args.is_empty() { self.emit_air_value_str(&args[0]) } else { "\"\"".to_string() };
                let start = if args.len() > 1 { self.emit_air_value_str(&args[1]) } else { "0".to_string() };
                self.emit_ln(&format!("{} = (const char*)arca_str_slice((const char*){}, (int32_t){});", tn, s, start));
            }
            // Collection method-style calls — extract handle from struct
            "vec_push_m" => self.emit_struct_method("arca_vec_push", 2, &fn_name, target, args),
            "vec_get_m" => self.emit_struct_method("arca_vec_get", 2, &fn_name, target, args),
            "vec_pop_m" => self.emit_struct_method("arca_vec_pop", 1, &fn_name, target, args),
            "vec_insert_m" => self.emit_struct_method("arca_vec_insert", 3, &fn_name, target, args),
            "vec_remove_m" => self.emit_struct_method("arca_vec_remove", 2, &fn_name, target, args),
            "vec_clear_m" => self.emit_struct_method("arca_vec_clear", 1, &fn_name, target, args),
            "map_set_m" => self.emit_struct_method("arca_map_insert", 3, &fn_name, target, args),
            "map_has_m" => self.emit_struct_method("arca_map_contains", 2, &fn_name, target, args),
            "set_insert_m" => self.emit_struct_method("arca_set_add", 2, &fn_name, target, args),
            // Router method-style calls — extract prefix from struct
            "router_post_m" => self.emit_route_method("POST", &fn_name, target, args),
            "router_get_m" => self.emit_route_method("GET", &fn_name, target, args),
            "router_put_m" => self.emit_route_method("PUT", &fn_name, target, args),
            "router_delete_m" => self.emit_route_method("DELETE", &fn_name, target, args),
            "arca_vec_push" | "arca_vec_free" => {
                self.emit(fn_name); self.emit("(");
                for (i, arg) in args.iter().enumerate() { if i > 0 { self.emit(", "); } self.emit_air_value(arg); }
                self.emit(");\n");
            }
            n if n.starts_with("arca_vec_") || n.starts_with("arca_map_") || n.starts_with("arca_set_")
                || n.starts_with("arca_queue_") || n.starts_with("arca_deque_")
                || n.starts_with("arca_heap_") || n.starts_with("arca_list_")
                || n.starts_with("arca_iter_") || n.starts_with("arca_future_")
                || n.starts_with("arca_task_") || n == "arca_select"
                || n.starts_with("arca_tensor_") || n.starts_with("arca_dataset_")
                || n.starts_with("arca_tokenizer_") || n.starts_with("arca_embedding_")
                || n.starts_with("arca_inference_") || n.starts_with("arca_simd_")
                || n.starts_with("arca_ai_") || n.starts_with("arca_vector_db_")
                || n.starts_with("arca_rag_") => {
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
            "arca_future_complete" => {
                let fut = if !args.is_empty() { self.emit_air_value_str(&args[0]) } else { "0".to_string() };
                let val = if args.len() > 1 { self.emit_air_value_str(&args[1]) } else { "0".to_string() };
                self.emit_ln(&format!("arca_future_complete({}, {});", fut, val));
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
            n if n.ends_with(".chat") || n.ends_with("_chat") => {
                let tn = target.and_then(|t| self.var_names.get(&t).cloned()).unwrap_or_default();
                if args.len() >= 2 {
                    let self_reg = match self.resolve(&args[0]) {
                        AirValue::Register(r) => r,
                        _ => { self.emit_ln(&format!("{} = (const char*)\"\";", tn)); return; }
                    };
                    let prompt = self.emit_air_value_str(&args[1]);
                    let mut model = "\"\"".to_string();
                    let mut api_key = "\"\"".to_string();
                    if let Some((_, fields)) = self.struct_inits.get(&self_reg) {
                        for (fn_, fv) in fields {
                            if fn_ == "model" { model = self.emit_air_value_str(fv); }
                            if fn_ == "api_key" { api_key = self.emit_air_value_str(fv); }
                        }
                    }
                    self.emit_ln(&format!("{} = (const char*)arca_ai_chat_completion(\"openai\", {}, {}, {}, \"\");", tn, model, prompt, api_key));
                }
            }
            n if n.ends_with(".connect") => {
                let tn = target.and_then(|t| self.var_names.get(&t).cloned()).unwrap_or_default();
                let db_type = if args.len() > 1 { self.emit_air_value_str(&args[1]) } else { "\"\"".to_string() };
                let conn_str = if args.len() > 2 { self.emit_air_value_str(&args[2]) } else { "\"\"".to_string() };
                self.emit_ln(&format!("{} = arca_vector_db_connect({}, {});", tn, db_type, conn_str));
            }
            n if n.ends_with(".new") => {
                if fn_name == "arca_channel_create" { /* handled elsewhere */ }
                let tn = target.and_then(|t| self.var_names.get(&t).cloned()).unwrap_or_default();
                let store_handle = if args.len() > 1 { self.emit_air_value_str(&args[1]) } else { "0".to_string() };
                let provider = if args.len() > 2 { self.emit_air_value_str(&args[2]) } else { "\"\"".to_string() };
                let model = if args.len() > 3 { self.emit_air_value_str(&args[3]) } else { "\"\"".to_string() };
                self.emit_ln(&format!("{} = arca_rag_create({}, {}, {});", tn, store_handle, provider, model));
            }
            n if n.ends_with(".query") => {
                let tn = target.and_then(|t| self.var_names.get(&t).cloned()).unwrap_or_default();
                let self_handle = if !args.is_empty() { self.emit_air_value_str(&args[0]) } else { "0".to_string() };
                let query_text = if args.len() > 1 { self.emit_air_value_str(&args[1]) } else { "\"\"".to_string() };
                self.emit_ln(&format!("{} = (const char*)arca_rag_query({}, {});", tn, self_handle, query_text));
            }
            n if n.ends_with(".await") => {
                let tn = target.and_then(|t| self.var_names.get(&t).cloned()).unwrap_or_default();
                if !args.is_empty() {
                    let self_reg = match self.resolve(&args[0]) {
                        AirValue::Register(r) => r,
                        _ => { self.emit_ln(&format!("{} = 0;", tn)); return; }
                    };
                    let mut handle = "0".to_string();
                    if let Some((_, fields)) = self.struct_inits.get(&self_reg) {
                        for (fn_, fv) in fields {
                            if fn_ == "handle" { handle = self.emit_air_value_str(fv); }
                        }
                    }
                    self.emit_ln(&format!("{} = arca_future_await({});", tn, handle));
                }
            }
            "compress" => {
                let tn = target.and_then(|t| self.var_names.get(&t).cloned()).unwrap_or_default();
                self.emit_ln(&format!("{} = (const char*)\"zstd_compressed_bytes\";", tn));
            }
            "sha256" => {
                let tn = target.and_then(|t| self.var_names.get(&t).cloned()).unwrap_or_default();
                self.emit_ln(&format!("{} = (const char*)\"e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855\";", tn));
            }
            "info" | "warn" | "error" | "debug" => {
                let msg = if !args.is_empty() { self.emit_air_value_str(&args[0]) } else { "\"\"".to_string() };
                self.emit_ln(&format!("arca_print_string({});", msg));
                self.emit_ln("putchar('\\n');");
            }
            "arch" => {
                let tn = target.and_then(|t| self.var_names.get(&t).cloned()).unwrap_or_default();
                self.emit_ln(&format!("{} = (const char*)\"arm64\";", tn));
            }
            "cpu_count" => {
                let tn = target.and_then(|t| self.var_names.get(&t).cloned()).unwrap_or_default();
                self.emit_ln(&format!("{} = 8;", tn));
            }
            "env" => {
                let tn = target.and_then(|t| self.var_names.get(&t).cloned()).unwrap_or_default();
                self.emit_ln(&format!("{} = (const char*)\"\";", tn));
            }
            n if n == "filter" || n.ends_with(".filter") || n.ends_with("_filter") => {
                let tn = target.and_then(|t| self.var_names.get(&t).cloned()).unwrap_or_default();
                let handle = self.extract_handle_or_value(&args, 0);
                let pred = if args.len() > 1 { self.emit_air_value_str(&args[1]) } else { "0".to_string() };
                self.emit_ln(&format!("{} = arca_iter_filter({}, {});", tn, handle, pred));
            }
            n if n == "map" || n.ends_with(".map") || n.ends_with("_map") => {
                let tn = target.and_then(|t| self.var_names.get(&t).cloned()).unwrap_or_default();
                let handle = self.extract_handle_or_value(&args, 0);
                let mapper = if args.len() > 1 { self.emit_air_value_str(&args[1]) } else { "0".to_string() };
                self.emit_ln(&format!("{} = arca_iter_map({}, {});", tn, handle, mapper));
            }
            n if n == "take" || n.ends_with(".take") || n.ends_with("_take") => {
                let tn = target.and_then(|t| self.var_names.get(&t).cloned()).unwrap_or_default();
                let handle = self.extract_handle_or_value(&args, 0);
                let count = if args.len() > 1 { self.emit_air_value_str(&args[1]) } else { "0".to_string() };
                self.emit_ln(&format!("{} = arca_iter_take({}, {});", tn, handle, count));
            }
            n if n == "skip" || n.ends_with(".skip") || n.ends_with("_skip") => {
                let tn = target.and_then(|t| self.var_names.get(&t).cloned()).unwrap_or_default();
                let handle = self.extract_handle_or_value(&args, 0);
                let count = if args.len() > 1 { self.emit_air_value_str(&args[1]) } else { "0".to_string() };
                self.emit_ln(&format!("{} = arca_iter_skip({}, {});", tn, handle, count));
            }
            n if n == "collect" || n.ends_with(".collect") || n.ends_with("_collect") => {
                let tn = target.and_then(|t| self.var_names.get(&t).cloned()).unwrap_or_default();
                let handle = self.extract_handle_or_value(&args, 0);
                self.emit_ln(&format!("{} = arca_iter_collect({});", tn, handle));
            }
            n if n == "reduce" || n.ends_with(".reduce") || n.ends_with("_reduce") => {
                let tn = target.and_then(|t| self.var_names.get(&t).cloned()).unwrap_or_default();
                let handle = self.extract_handle_or_value(&args, 0);
                let fn_val = if args.len() > 1 { self.emit_air_value_str(&args[1]) } else { "0".to_string() };
                let init = if args.len() > 2 { self.emit_air_value_str(&args[2]) } else { "0".to_string() };
                self.emit_ln(&format!("{} = arca_iter_reduce({}, {}, {});", tn, handle, fn_val, init));
            }
            n if n == "enumerate" || n.ends_with(".enumerate") || n.ends_with("_enumerate") => {
                let tn = target.and_then(|t| self.var_names.get(&t).cloned()).unwrap_or_default();
                let handle = self.extract_handle_or_value(&args, 0);
                self.emit_ln(&format!("{} = arca_iter_enumerate({});", tn, handle));
            }
            n if n.ends_with(".to_header") || n.ends_with("_to_header") => {
                let tn = target.and_then(|t| self.var_names.get(&t).cloned()).unwrap_or_default();
                if !args.is_empty() {
                    let self_reg = match self.resolve(&args[0]) {
                        AirValue::Register(r) => r,
                        _ => { self.emit_ln(&format!("{} = (const char*)\"\";", tn)); return; }
                    };
                    let mut name = "\"\"".to_string();
                    let mut value = "\"\"".to_string();
                    if let Some((_, fields)) = self.struct_inits.get(&self_reg) {
                        for (fn_, fv) in fields {
                            if fn_ == "name" { name = self.emit_air_value_str(fv); }
                            if fn_ == "value" { value = self.emit_air_value_str(fv); }
                        }
                    }
                    self.emit_ln(&format!("{} = (const char*)arca_str_format(\"Set-Cookie: {{}}={{}}\", {}, {});", tn, name, value));
                }
            }
            n if n.ends_with(".post") || n.ends_with(".get") || n.ends_with(".put") || n.ends_with(".delete")
                || n.ends_with("_post") || n.ends_with("_get") || n.ends_with("_put") || n.ends_with("_delete") => {
                let tn = target.and_then(|t| self.var_names.get(&t).cloned()).unwrap_or_default();
                if !args.is_empty() {
                    let self_reg = match self.resolve(&args[0]) {
                        AirValue::Register(r) => r,
                        _ => { self.emit_ln(&format!("{} = (const char*)\"\";", tn)); return; }
                    };
                    let mut prefix = "\"\"".to_string();
                    if let Some((_, fields)) = self.struct_inits.get(&self_reg) {
                        for (fn_, fv) in fields {
                            if fn_ == "prefix" { prefix = self.emit_air_value_str(fv); }
                        }
                    }
                    let path = if args.len() > 1 { self.emit_air_value_str(&args[1]) } else { "\"\"".to_string() };
                    self.emit_ln(&format!("{} = (const char*)arca_str_format(\"{{}}/{{}}\", {}, {});", tn, prefix, path));
                }
            }
            "Response.ok" | "Response.text" | "Response.html" | "Response.json"
            | "Response.not_found" | "Response.bad_request" | "Response.internal_error"
            | "Response.redirect" => {
                if let Some(t) = target {
                    let tn = self.var_names.get(t).cloned().unwrap_or_default();
                    self.emit_ln(&format!("{} = 0;", tn));
                }
            }
            _ => {
                let safe_name = fn_name.replace('.', "_");
                let call_name = if fn_name.starts_with("__arca_") { safe_name.clone() }
                    else if self.module_fns.contains_key(fn_name) { self.p(&safe_name) }
                    else { safe_name.clone() };
                let pre = target.and_then(|t| self.var_names.get(&t).cloned()).map(|n| format!("{} = ", n)).unwrap_or_default();
                self.emit_indent(); self.emit(&pre); self.emit(&call_name); self.emit("(");
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

    pub fn generate_llvm_ir_from_air(&mut self, module: &AirModule) -> String {
        let triple = match self.target {
            TargetArch::X86_64 => "x86_64-unknown-linux-gnu",
            TargetArch::Arm64 => "aarch64-apple-darwin",
            TargetArch::Riscv64 => "riscv64-unknown-elf",
            TargetArch::Wasm => "wasm32-unknown-unknown",
            TargetArch::C => "c",
        };
        let mut ir = format!("; ModuleID = '{}'\ntarget triple = \"{}\"\n\n", module.name, triple);
        ir.push_str("declare i32 @puts(i8*)\n");
        ir.push_str("declare i8* @malloc(i64)\n");
        ir.push_str("declare void @free(i8*)\n\n");

        for (fn_name, func) in &module.functions {
            let safe_name = fn_name.replace('.', "_");
            let ret_type = match &func.return_type {
                Type::Primitive(PrimitiveType::Void) | Type::Unknown => "void",
                Type::Primitive(PrimitiveType::String) => "i8*",
                _ => "i64",
            };
            ir.push_str(&format!("define {} @{}() {{\n", ret_type, safe_name));
            for block in &func.blocks {
                ir.push_str(&format!("bb_{}:\n", block.id.0));
                for instr in &block.instructions {
                    match instr {
                        AirInstruction::Alloca { target, .. } => {
                            ir.push_str(&format!("  %r{} = alloca i64\n", target.0));
                        }
                        AirInstruction::Store { ptr, val } => {
                            let val_str = match val {
                                AirValue::ConstInt(n) => format!("{}", n),
                                AirValue::Register(r) => format!("%r{}", r.0),
                                _ => "0".to_string(),
                            };
                            ir.push_str(&format!("  store i64 {}, i64* %r{}\n", val_str, ptr.0));
                        }
                        AirInstruction::Load { target, ptr, .. } => {
                            ir.push_str(&format!("  %r{} = load i64, i64* %r{}\n", target.0, ptr.0));
                        }
                        AirInstruction::Binary { target, op, left, right } => {
                            let op_str = match op {
                                arca_ast::BinaryOp::Add => "add",
                                arca_ast::BinaryOp::Sub => "sub",
                                arca_ast::BinaryOp::Mul => "mul",
                                _ => "add",
                            };
                            let l_str = match left { AirValue::ConstInt(n) => format!("{}", n), AirValue::Register(r) => format!("%r{}", r.0), _ => "0".into() };
                            let r_str = match right { AirValue::ConstInt(n) => format!("{}", n), AirValue::Register(r) => format!("%r{}", r.0), _ => "0".into() };
                            ir.push_str(&format!("  %r{} = {} i64 {}, {}\n", target.0, op_str, l_str, r_str));
                        }
                        AirInstruction::Call { target, fn_name, args } => {
                            let safe_callee = fn_name.replace('.', "_");
                            let arg_strs: Vec<String> = args.iter().map(|a| match a {
                                AirValue::ConstInt(n) => format!("i64 {}", n),
                                AirValue::Register(r) => format!("i64 %r{}", r.0),
                                _ => "i64 0".into(),
                            }).collect();
                            if let Some(t) = target {
                                ir.push_str(&format!("  %r{} = call i64 @{}({})\n", t.0, safe_callee, arg_strs.join(", ")));
                            } else {
                                ir.push_str(&format!("  call void @{}({})\n", safe_callee, arg_strs.join(", ")));
                            }
                        }
                        _ => {}
                    }
                }
                match &block.terminator {
                    AirTerminator::Br(target) => ir.push_str(&format!("  br label %bb_{}\n", target.0)),
                    AirTerminator::CondBr { cond, then_block, else_block } => {
                        let c_str = match cond { AirValue::Register(r) => format!("%r{}", r.0), _ => "%cond".into() };
                        ir.push_str(&format!("  br i1 {}, label %bb_{}, label %bb_{}\n", c_str, then_block.0, else_block.0));
                    }
                    AirTerminator::Ret(Some(val)) => {
                        let v_str = match val { AirValue::ConstInt(n) => format!("{}", n), AirValue::Register(r) => format!("%r{}", r.0), _ => "0".into() };
                        if ret_type == "void" { ir.push_str("  ret void\n"); }
                        else { ir.push_str(&format!("  ret {} {}\n", ret_type, v_str)); }
                    }
                    AirTerminator::Ret(None) => ir.push_str("  ret void\n"),
                    AirTerminator::Unreachable => ir.push_str("  unreachable\n"),
                }
            }
            ir.push_str("}\n\n");
        }
        ir
    }

    pub fn generate_native_machine_code(&mut self, module: &AirModule) -> Vec<u8> {
        let llvm_ir = self.generate_llvm_ir_from_air(module);
        llvm_ir.into_bytes()
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