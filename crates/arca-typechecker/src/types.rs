//! Type definitions for the Arca type system.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum PrimitiveType {
    I8,
    I16,
    I32,
    I64,
    U8,
    U16,
    U32,
    U64,
    F32,
    F64,
    Bool,
    String,
    Char,
    Void,
}

impl fmt::Display for PrimitiveType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PrimitiveType::I8 => write!(f, "i8"),
            PrimitiveType::I16 => write!(f, "i16"),
            PrimitiveType::I32 => write!(f, "i32"),
            PrimitiveType::I64 => write!(f, "i64"),
            PrimitiveType::U8 => write!(f, "u8"),
            PrimitiveType::U16 => write!(f, "u16"),
            PrimitiveType::U32 => write!(f, "u32"),
            PrimitiveType::U64 => write!(f, "u64"),
            PrimitiveType::F32 => write!(f, "f32"),
            PrimitiveType::F64 => write!(f, "f64"),
            PrimitiveType::Bool => write!(f, "bool"),
            PrimitiveType::String => write!(f, "string"),
            PrimitiveType::Char => write!(f, "char"),
            PrimitiveType::Void => write!(f, "void"),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FnType {
    pub params: Vec<Type>,
    pub return_type: Box<Type>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Type {
    Primitive(PrimitiveType),
    UntypedInt(i64),
    UntypedFloat(f64),
    Struct {
        name: String,
        fields: HashMap<String, Type>,
        methods: HashMap<String, FnType>,
    },
    Enum {
        name: String,
        variants: HashMap<String, Vec<Type>>,
    },
    Fn(FnType),
    Reference {
        is_mut: bool,
        inner: Box<Type>,
    },
    Option(Box<Type>),
    Result(Box<Type>, Box<Type>),
    ErrorUnion(Vec<Type>),
    None,
    Unknown,
}

impl Type {
    pub fn is_numeric(&self) -> bool {
        match self {
            Type::Primitive(p) => matches!(
                p,
                PrimitiveType::I8
                    | PrimitiveType::I16
                    | PrimitiveType::I32
                    | PrimitiveType::I64
                    | PrimitiveType::U8
                    | PrimitiveType::U16
                    | PrimitiveType::U32
                    | PrimitiveType::U64
                    | PrimitiveType::F32
                    | PrimitiveType::F64
            ),
            Type::UntypedInt(_) | Type::UntypedFloat(_) => true,
            _ => false,
        }
    }

    pub fn is_assignable_to(&self, target: &Type) -> bool {
        if self == target {
            return true;
        }

        match (self, target) {
            (Type::UntypedInt(_), Type::UntypedInt(_)) => true,
            (Type::UntypedInt(_), Type::Primitive(p)) => matches!(
                p,
                PrimitiveType::I8
                    | PrimitiveType::I16
                    | PrimitiveType::I32
                    | PrimitiveType::I64
                    | PrimitiveType::U8
                    | PrimitiveType::U16
                    | PrimitiveType::U32
                    | PrimitiveType::U64
            ),
            (Type::UntypedFloat(_), Type::UntypedFloat(_)) => true,
            (Type::UntypedFloat(_), Type::Primitive(p)) => {
                matches!(p, PrimitiveType::F32 | PrimitiveType::F64)
            }
            (Type::Unknown, _) | (_, Type::Unknown) => true,
            (Type::None, Type::Reference { .. }) => true,
            (Type::None, Type::Option(_)) => true,
            (Type::None, Type::Primitive(PrimitiveType::Void)) => true,
            (Type::Reference { .. }, Type::Primitive(PrimitiveType::Void)) => true,
            (Type::Primitive(PrimitiveType::String), Type::Primitive(PrimitiveType::Void)) => true,
            _ => false,
        }
    }
}

impl fmt::Display for Type {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Type::Primitive(p) => write!(f, "{}", p),
            Type::UntypedInt(n) => write!(f, "untyped int({})", n),
            Type::UntypedFloat(fl) => write!(f, "untyped float({})", fl),
            Type::Struct { name, .. } => write!(f, "struct {}", name),
            Type::Enum { name, .. } => write!(f, "enum {}", name),
            Type::Fn(fnt) => {
                let params_str: Vec<String> = fnt.params.iter().map(|p| format!("{}", p)).collect();
                write!(f, "fn({}) -> {}", params_str.join(", "), fnt.return_type)
            }
            Type::Reference { is_mut, inner } => {
                if *is_mut {
                    write!(f, "ptr<{}>", inner)
                } else {
                    write!(f, "ref<{}>", inner)
                }
            }
            Type::Option(inner) => write!(f, "Option<{}>", inner),
            Type::Result(ok, err) => write!(f, "Result<{}, {}>", ok, err),
            Type::ErrorUnion(variants) => {
                let strs: Vec<String> = variants.iter().map(|v| format!("{}", v)).collect();
                write!(f, "{}", strs.join(" | "))
            }
            Type::None => write!(f, "none"),
            Type::Unknown => write!(f, "Unknown"),
        }
    }
}
