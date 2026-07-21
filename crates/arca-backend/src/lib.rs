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

    fn air_type_to_c(&self, ty: &Type) -> &str {
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
            _ => "int64_t",
        }
    }

    fn type_for_instr(&self, instr: &AirInstruction) -> String {
        match instr {
            AirInstruction::Alloca { ty, .. } => self.air_type_to_c(ty).to_string(),
            AirInstruction::Load { .. } => "int64_t".to_string(),
            AirInstruction::Binary { op, .. } => match op {
                BinaryOp::Equal | BinaryOp::NotEqual | BinaryOp::Less | BinaryOp::LessEqual
                | BinaryOp::Greater | BinaryOp::GreaterEqual | BinaryOp::And | BinaryOp::Or => "bool".to_string(),
                _ => "int64_t".to_string(),
            }
            AirInstruction::Call { fn_name, .. } => {
                if fn_name.starts_with("arca_std_") || fn_name == "arca_time_ns"
                    || fn_name == "serve" || fn_name == "arca_parse_int"
                    || fn_name == "arca_strcmp" || fn_name == "arca_starts_with"
                    || fn_name == "arca_str_rfind" || fn_name == "__arca_parse_int"
                    || fn_name == "__arca_starts_with" || fn_name == "__arca_str_rfind"
                {
                    "int64_t".to_string()
                } else if fn_name == "println" || fn_name == "print" || fn_name.starts_with("show_") {
                    "void".to_string()
                } else if fn_name.ends_with("_to_str") || fn_name.ends_with("user_json") || fn_name.ends_with("users_json")
                    || fn_name.starts_with("build_") || fn_name == "hash" || fn_name == "list_users" {
                    "const char*".to_string()
                } else {
                    "int64_t".to_string()
                }
            }
            AirInstruction::StructInit { struct_name, .. } => {
                if struct_name.is_empty() { "int64_t".to_string() } else { struct_name.clone() }
            }
            AirInstruction::FieldLoad { .. } => "const char*".to_string(),
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

        // Collect struct definitions from all StructInit instructions
        let mut struct_defs: HashMap<String, Vec<String>> = HashMap::new();
        for func in module.functions.values() {
            for block in &func.blocks {
                for instr in &block.instructions {
                    if let AirInstruction::StructInit { struct_name, fields, .. } = instr {
                        if !struct_name.is_empty() && !struct_defs.contains_key(struct_name) {
                            let field_names: Vec<String> = fields.iter().map(|(n, _)| n.clone()).collect();
                            struct_defs.insert(struct_name.clone(), field_names);
                        }
                    }
                }
            }
        }

        // Emit struct type definitions
        for (name, field_names) in &struct_defs {
            self.emit(&format!("typedef struct {{ /* {} */", name));
            for fn_ in field_names {
                self.emit(&format!("int64_t {}; ", fn_));
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
                            && fn_name != "println" && fn_name != "print"
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

        // Pre-register param names
        for (reg, cname) in &param_reg_names {
            self.var_names.insert(*reg, cname.clone());
        }

        // First pass: declare types and build copy-propagation map
        // Track which Alloca ptrs are stored with a Register (copy source)
        let mut store_sources: HashMap<RegisterId, AirValue> = HashMap::new();
        let mut reg_decls: Vec<(RegisterId, String)> = Vec::new();

        for block in &func.blocks {
            for instr in &block.instructions {
                if let AirInstruction::Store { ptr, val } = instr {
                    if matches!(val, AirValue::Register(_)) {
                        store_sources.insert(*ptr, val.clone());
                    }
                }
            }
        }

        // Second pass: for each Load, if ptr was stored from a Register, it's a copy
        for block in &func.blocks {
            for instr in &block.instructions {
                if let AirInstruction::Load { target, ptr, .. } = instr {
                    if let Some(src) = store_sources.get(ptr) {
                        self.copy_sources.insert(*target, src.clone());
                        continue;
                    }
                }
                // Collect declarations for non-param, non-copy registers
                let reg = match instr {
                    AirInstruction::Alloca { target, .. } => Some(*target),
                    AirInstruction::Load { target, .. } => {
                        if self.copy_sources.contains_key(target) { None } else { Some(*target) }
                    }
                    AirInstruction::Binary { target, .. } => Some(*target),
                    AirInstruction::Call { target, fn_name, .. } => {
                        // Skip declarations for void-returning calls
                        let is_void = fn_name == "println" || fn_name == "print" || fn_name.starts_with("show_");
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
                let pn = self.var_names.get(ptr).cloned().unwrap_or_default();
                self.emit_indent();
                self.emit(&pn);
                self.emit(" = ");
                self.emit_air_value(&resolved);
                self.emit(";\n");
            }
            AirInstruction::Load { target, ptr, .. } => {
                // Copy-propagated loads are skipped entirely
                if self.copy_sources.contains_key(target) { return; }
                let tn = self.var_names.get(target).cloned().unwrap_or_default();
                let sn = self.var_names.get(ptr).cloned().unwrap_or_default();
                self.emit_ln(&format!("{} = {};", tn, sn));
            }
            AirInstruction::Binary { target, op, left, right } => {
                let tn = self.var_names.get(target).cloned().unwrap_or_default();
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
                    self.emit_indent(); self.emit(&tn); self.emit(" = arca_strcmp(");
                    self.emit_air_value(left); self.emit(", "); self.emit_air_value(right);
                    self.emit(&format!(") {} 0;\n", if is_not { "!=" } else { "==" }));
                } else if matches!(op, BinaryOp::Add) {
                    // String concatenation: if either operand is a string
                    let l_is_str = is_string_value(left, &self.var_types);
                    let r_is_str = is_string_value(right, &self.var_types);
                    if l_is_str || r_is_str {
                        self.emit_indent(); self.emit(&tn); self.emit(" = (int64_t)arca_strcat((const char*)");
                        self.emit_air_value(left); self.emit(", (const char*)");
                        self.emit_air_value(right); self.emit(");\n");
                        // Update var_types so subsequent uses know this is a string
                        self.var_types.insert(*target, "const char*".to_string());
                    } else {
                        self.emit_indent(); self.emit(&tn); self.emit(" = ");
                        self.emit_air_value(left); self.emit(" + "); self.emit_air_value(right); self.emit(";\n");
                    }
                } else {
                    self.emit_indent(); self.emit(&tn); self.emit(" = ");
                    self.emit_air_value(left); self.emit(os); self.emit_air_value(right); self.emit(";\n");
                }
            }
            AirInstruction::Call { target, fn_name, args } => self.emit_air_call(target, fn_name, args),
            AirInstruction::StructInit { target, struct_name, fields } => {
                let tn = self.var_names.get(target).cloned().unwrap_or_default();
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
                let tn = self.var_names.get(target).cloned().unwrap_or_default();
                let on = self.var_names.get(object).cloned().unwrap_or_default();
                // If the object was created by a StructInit, use struct type for field access
                if let Some((sname, _)) = self.struct_inits.get(object) {
                    if !sname.is_empty() {
                        self.emit_ln(&format!("{} = (({}*)&{})->{};", tn, sname, on, field));
                    } else {
                        self.emit_ln(&format!("{} = *((int64_t*)((char*)&{} + offsetof_placeholder_{}));", tn, on, field));
                    }
                } else {
                    self.emit_ln(&format!("{} = {}._field_{};", tn, on, field));
                }
            }
            AirInstruction::Ref { target, source } => {
                let tn = self.var_names.get(target).cloned().unwrap_or_default();
                let sn = self.var_names.get(source).cloned().unwrap_or_default();
                self.emit_ln(&format!("{} = &{};", tn, sn));
            }
            AirInstruction::Deref { target, ptr } => {
                let tn = self.var_names.get(target).cloned().unwrap_or_default();
                let pn = self.var_names.get(ptr).cloned().unwrap_or_default();
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
            "__arca_int_to_str" => {
                let tn = target.and_then(|t| self.var_names.get(&t).cloned()).unwrap_or_default();
                let v = if !args.is_empty() { self.emit_air_value_str(&args[0]) } else { "0".to_string() };
                self.emit_ln(&format!("{} = arca_int_to_str({});", tn, v));
            }
            "__arca_starts_with" => {
                let tn = target.and_then(|t| self.var_names.get(&t).cloned()).unwrap_or_default();
                let s = if args.len() > 0 { self.emit_air_value_str(&args[0]) } else { "\"\"".to_string() };
                let p = if args.len() > 1 { self.emit_air_value_str(&args[1]) } else { "\"\"".to_string() };
                self.emit_ln(&format!("{} = arca_starts_with({}, {});", tn, s, p));
            }
            "__arca_parse_int" => {
                let tn = target.and_then(|t| self.var_names.get(&t).cloned()).unwrap_or_default();
                let s = if !args.is_empty() { self.emit_air_value_str(&args[0]) } else { "\"0\"".to_string() };
                self.emit_ln(&format!("{} = arca_parse_int({});", tn, s));
            }
            "__arca_str_rfind" => {
                let tn = target.and_then(|t| self.var_names.get(&t).cloned()).unwrap_or_default();
                let s = if args.len() > 0 { self.emit_air_value_str(&args[0]) } else { "\"\"".to_string() };
                let c = if args.len() > 1 { self.emit_air_value_str(&args[1]) } else { "0".to_string() };
                self.emit_ln(&format!("{} = arca_str_rfind({}, {});", tn, s, c));
            }
            "__arca_str_slice" => {
                let tn = target.and_then(|t| self.var_names.get(&t).cloned()).unwrap_or_default();
                let s = if args.len() > 0 { self.emit_air_value_str(&args[0]) } else { "\"\"".to_string() };
                let start = if args.len() > 1 { self.emit_air_value_str(&args[1]) } else { "0".to_string() };
                self.emit_ln(&format!("{} = arca_str_slice({}, {});", tn, s, start));
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