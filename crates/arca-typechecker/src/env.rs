//! Type checking environment and symbol scope stack.

use crate::types::{FnType, PrimitiveType, Type};
use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct Scope {
    pub bindings: HashMap<String, Type>,
}

impl Scope {
    pub fn new() -> Self {
        Self {
            bindings: HashMap::new(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TypeEnv {
    scopes: Vec<Scope>,
    pub structs: HashMap<String, Type>,
    pub functions: HashMap<String, FnType>,
    pub current_struct: Option<String>,
}

impl TypeEnv {
    pub fn new() -> Self {
        let mut env = Self {
            scopes: vec![Scope::new()],
            structs: HashMap::new(),
            functions: HashMap::new(),
            current_struct: None,
        };
        env.register_primitives();
        env
    }

    fn register_primitives(&mut self) {
        let mut arena_methods = HashMap::new();
        arena_methods.insert(
            "new".into(),
            FnType {
                params: Vec::new(),
                return_type: Box::new(Type::Struct {
                    name: "Arena".into(),
                    fields: HashMap::new(),
                    methods: HashMap::new(),
                }),
            },
        );

        let arena_struct = Type::Struct {
            name: "Arena".into(),
            fields: HashMap::new(),
            methods: arena_methods,
        };

        self.structs.insert("Arena".into(), arena_struct);

        let void_fn = FnType {
            params: Vec::new(),
            return_type: Box::new(Type::Primitive(PrimitiveType::Void)),
        };

        self.functions.insert("generateTable".into(), void_fn.clone());
        self.functions.insert("process".into(), void_fn.clone());
        self.functions.insert("println".into(), void_fn.clone());
        self.functions.insert("print".into(), void_fn.clone());
        self.functions.insert("panic".into(), void_fn.clone());
        self.functions.insert("assert".into(), void_fn.clone());

        let int_fn = FnType {
            params: vec![Type::Primitive(PrimitiveType::I32)],
            return_type: Box::new(Type::Primitive(PrimitiveType::I32)),
        };
        self.functions.insert("serve".into(), int_fn.clone());
        self.functions.insert("arca_std_http_serve".into(), int_fn.clone());

        let time_fn = FnType {
            params: Vec::new(),
            return_type: Box::new(Type::Primitive(PrimitiveType::I64)),
        };
        self.functions.insert("Instant.now".into(), time_fn.clone());
        self.functions.insert("now".into(), time_fn.clone());
        self.functions.insert("elapsed_ms".into(), time_fn.clone());
        self.functions.insert("elapsed_ns".into(), time_fn.clone());

        // Result/Option constructors
        let i64_to_i64 = FnType {
            params: vec![Type::Primitive(PrimitiveType::I64)],
            return_type: Box::new(Type::Primitive(PrimitiveType::I64)),
        };
        self.functions.insert("Ok".into(), i64_to_i64.clone());
        self.functions.insert("Err".into(), i64_to_i64.clone());
        self.functions.insert("Some".into(), i64_to_i64.clone());
        self.functions.insert("arca_result_ok".into(), i64_to_i64.clone());
        self.functions.insert("arca_result_err".into(), i64_to_i64.clone());
        self.functions.insert("arca_option_some".into(), i64_to_i64.clone());
        self.functions.insert("__arca_throw".into(), i64_to_i64.clone());

        let i64_to_i32 = FnType {
            params: vec![Type::Primitive(PrimitiveType::I64)],
            return_type: Box::new(Type::Primitive(PrimitiveType::I32)),
        };
        self.functions.insert("arca_result_is_ok".into(), i64_to_i32.clone());
        self.functions.insert("arca_option_is_some".into(), i64_to_i32.clone());
        self.functions.insert("arca_result_unwrap".into(), i64_to_i64.clone());

        let void_to_i64 = FnType {
            params: Vec::new(),
            return_type: Box::new(Type::Primitive(PrimitiveType::I64)),
        };
        self.functions.insert("__arca_get_last_error".into(), void_to_i64.clone());

        let void_to_void = FnType {
            params: Vec::new(),
            return_type: Box::new(Type::Primitive(PrimitiveType::Void)),
        };
        self.functions.insert("__arca_clear_last_error".into(), void_to_void.clone());

        // std/os module
        let string_fn = FnType {
            params: vec![Type::Primitive(PrimitiveType::String)],
            return_type: Box::new(Type::Primitive(PrimitiveType::String)),
        };
        self.functions.insert("compress".into(), string_fn.clone());
        self.functions.insert("sha256".into(), string_fn.clone());
        let info_fn = FnType {
            params: vec![Type::Primitive(PrimitiveType::String)],
            return_type: Box::new(Type::Primitive(PrimitiveType::Void)),
        };
        self.functions.insert("info".into(), info_fn);
        let os_arch = FnType {
            params: Vec::new(),
            return_type: Box::new(Type::Primitive(PrimitiveType::String)),
        };
        self.functions.insert("arch".into(), os_arch.clone());
        self.functions.insert("cpu_count".into(), time_fn.clone());
        self.functions.insert("env".into(), FnType {
            params: vec![Type::Primitive(PrimitiveType::String)],
            return_type: Box::new(Type::Primitive(PrimitiveType::I64)),
        });

        // Standard library module bindings
        self.insert_var("serve".into(), Type::Unknown);
        self.insert_var("log".into(), Type::Unknown);
        self.insert_var("crypto".into(), Type::Unknown);
        self.insert_var("gzip".into(), Type::Unknown);
        self.insert_var("zstd".into(), Type::Unknown);
        self.insert_var("math".into(), Type::Unknown);
        self.insert_var("mem".into(), Type::Unknown);
        self.insert_var("hash".into(), Type::Unknown);
        self.insert_var("json".into(), Type::Unknown);
        self.insert_var("os".into(), Type::Unknown);
        self.insert_var("process".into(), Type::Unknown);
        self.insert_var("time".into(), Type::Unknown);
        self.insert_var("Process".into(), Type::Unknown);
        self.insert_var("Instant".into(), Type::Unknown);
        self.insert_var("Duration".into(), Type::Unknown);
        self.insert_var("ArenaAllocator".into(), Type::Unknown);
        self.insert_var("Response".into(), Type::Unknown);
        self.insert_var("Request".into(), Type::Unknown);
        self.insert_var("Pool".into(), Type::Unknown);
        self.insert_var("Arena".into(), Type::Unknown);
        self.insert_var("Router".into(), Type::Unknown);
        self.insert_var("File".into(), Type::Unknown);
        self.insert_var("TcpListener".into(), Type::Unknown);
        self.insert_var("Channel".into(), Type::Unknown);
        self.insert_var("Array".into(), Type::Unknown);
        self.insert_var("Map".into(), Type::Unknown);
        self.insert_var("Set".into(), Type::Unknown);

        // FFI / Native interop namespace
        self.insert_var("c".into(), Type::Unknown);
    }

    pub fn push_scope(&mut self) {
        self.scopes.push(Scope::new());
    }

    pub fn pop_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    pub fn insert_var(&mut self, name: String, ty: Type) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.bindings.insert(name, ty);
        }
    }

    pub fn lookup_var(&self, name: &str) -> Option<Type> {
        for scope in self.scopes.iter().rev() {
            if let Some(ty) = scope.bindings.get(name) {
                return Some(ty.clone());
            }
        }
        None
    }

    pub fn lookup_type_annotation(&self, type_str: &str) -> Type {
        match type_str {
            "i8" => Type::Primitive(PrimitiveType::I8),
            "i16" => Type::Primitive(PrimitiveType::I16),
            "i32" => Type::Primitive(PrimitiveType::I32),
            "i64" => Type::Primitive(PrimitiveType::I64),
            "u8" => Type::Primitive(PrimitiveType::U8),
            "u16" => Type::Primitive(PrimitiveType::U16),
            "u32" => Type::Primitive(PrimitiveType::U32),
            "u64" => Type::Primitive(PrimitiveType::U64),
            "f32" => Type::Primitive(PrimitiveType::F32),
            "f64" => Type::Primitive(PrimitiveType::F64),
            "bool" => Type::Primitive(PrimitiveType::Bool),
            "string" => Type::Primitive(PrimitiveType::String),
            "char" => Type::Primitive(PrimitiveType::Char),
            "void" => Type::Primitive(PrimitiveType::Void),
            "Self" => {
                if let Some(sname) = &self.current_struct {
                    if let Some(st) = self.structs.get(sname) {
                        st.clone()
                    } else {
                        Type::Unknown
                    }
                } else {
                    Type::Unknown
                }
            }
            "c_void_ptr" => Type::Primitive(PrimitiveType::Void),
            custom => {
                if let Some(st) = self.structs.get(custom) {
                    st.clone()
                } else {
                    Type::Unknown
                }
            }
        }
    }
}
