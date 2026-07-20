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
}

impl TypeEnv {
    pub fn new() -> Self {
        let mut env = Self {
            scopes: vec![Scope::new()],
            structs: HashMap::new(),
            functions: HashMap::new(),
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

        self.functions.insert(
            "generateTable".into(),
            FnType {
                params: Vec::new(),
                return_type: Box::new(Type::Primitive(PrimitiveType::Void)),
            },
        );
        self.functions.insert(
            "process".into(),
            FnType {
                params: Vec::new(),
                return_type: Box::new(Type::Primitive(PrimitiveType::Void)),
            },
        );
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
