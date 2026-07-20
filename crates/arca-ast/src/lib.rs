//! Abstract Syntax Tree definitions for the Arca programming language.

use serde::{Deserialize, Serialize};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Location {
    pub line: usize,
    pub column: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub start_loc: Location,
    pub end_loc: Location,
}

impl Span {
    pub fn new(start: usize, end: usize, start_loc: Location, end_loc: Location) -> Self {
        Self {
            start,
            end,
            start_loc,
            end_loc,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum LiteralKind {
    Int(i64),
    Float(f64),
    String(String),
    Char(char),
    Bool(bool),
    Null,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Rem,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    And,
    Or,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum UnaryOp {
    Neg,
    Not,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum TypeAnnotation {
    Named(String),
    Generic { name: String, args: Vec<TypeAnnotation> },
    Reference { is_mut: bool, inner: Box<TypeAnnotation> },
    Fn { params: Vec<TypeAnnotation>, return_type: Box<TypeAnnotation> },
}

impl fmt::Display for TypeAnnotation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TypeAnnotation::Named(name) => write!(f, "{}", name),
            TypeAnnotation::Generic { name, args } => {
                let args_str: Vec<String> = args.iter().map(|a| format!("{}", a)).collect();
                write!(f, "{}<{}>", name, args_str.join(", "))
            }
            TypeAnnotation::Reference { is_mut, inner } => {
                if *is_mut {
                    write!(f, "&mut {}", inner)
                } else {
                    write!(f, "&{}", inner)
                }
            }
            TypeAnnotation::Fn { params, return_type } => {
                let params_str: Vec<String> = params.iter().map(|p| format!("{}", p)).collect();
                write!(f, "({}) -> {}", params_str.join(", "), return_type)
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructFieldInit {
    pub name: String,
    pub value: Option<Expr>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BlockExpr {
    pub statements: Vec<Stmt>,
    pub final_expr: Option<Box<Expr>>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Pattern {
    Literal(LiteralKind),
    Identifier(String),
    Wildcard,
    Variant {
        enum_name: Option<String>,
        variant: String,
        inner: Vec<Pattern>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub guard: Option<Expr>,
    pub body: Expr,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Literal {
        value: LiteralKind,
        span: Span,
    },
    Identifier {
        name: String,
        span: Span,
    },
    Binary {
        left: Box<Expr>,
        op: BinaryOp,
        right: Box<Expr>,
        span: Span,
    },
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
        span: Span,
    },
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
        span: Span,
    },
    MemberAccess {
        object: Box<Expr>,
        property: String,
        is_optional: bool,
        span: Span,
    },
    NullCoalesce {
        left: Box<Expr>,
        right: Box<Expr>,
        span: Span,
    },
    StructLiteral {
        name: String,
        fields: Vec<StructFieldInit>,
        span: Span,
    },
    If {
        cond: Box<Expr>,
        then_branch: BlockExpr,
        else_branch: Option<Box<Expr>>,
        span: Span,
    },
    Match {
        value: Box<Expr>,
        arms: Vec<MatchArm>,
        span: Span,
    },
    Block(BlockExpr),
    ComptimeBlock {
        body: BlockExpr,
        span: Span,
    },
    SpawnBlock {
        body: BlockExpr,
        span: Span,
    },
    IntrinsicCall {
        name: String,
        args: Vec<Expr>,
        span: Span,
    },
}

impl Expr {
    pub fn span(&self) -> Span {
        match self {
            Expr::Literal { span, .. }
            | Expr::Identifier { span, .. }
            | Expr::Binary { span, .. }
            | Expr::Unary { span, .. }
            | Expr::Call { span, .. }
            | Expr::MemberAccess { span, .. }
            | Expr::NullCoalesce { span, .. }
            | Expr::StructLiteral { span, .. }
            | Expr::If { span, .. }
            | Expr::Match { span, .. }
            | Expr::ComptimeBlock { span, .. }
            | Expr::SpawnBlock { span, .. }
            | Expr::IntrinsicCall { span, .. } => *span,
            Expr::Block(b) => b.span,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    VarDecl {
        is_const: bool,
        name: String,
        type_ann: Option<TypeAnnotation>,
        init: Option<Expr>,
        span: Span,
    },
    Return {
        value: Option<Expr>,
        span: Span,
    },
    Defer {
        body: Box<Expr>,
        span: Span,
    },
    Expr {
        expr: Expr,
        has_semicolon: bool,
        span: Span,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FieldDef {
    pub name: String,
    pub type_ann: TypeAnnotation,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ParamDef {
    pub name: String,
    pub type_ann: TypeAnnotation,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FnDecl {
    pub name: String,
    pub params: Vec<ParamDef>,
    pub return_type: Option<TypeAnnotation>,
    pub throws_type: Option<TypeAnnotation>,
    pub body: BlockExpr,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnumVariantDef {
    pub name: String,
    pub payload: Vec<TypeAnnotation>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CapabilityMethodDef {
    pub name: String,
    pub params: Vec<ParamDef>,
    pub return_type: Option<TypeAnnotation>,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Decl {
    Struct {
        name: String,
        fields: Vec<FieldDef>,
        methods: Vec<FnDecl>,
        span: Span,
    },
    Extend {
        target_name: String,
        methods: Vec<FnDecl>,
        span: Span,
    },
    Enum {
        name: String,
        variants: Vec<EnumVariantDef>,
        span: Span,
    },
    Capability {
        name: String,
        methods: Vec<CapabilityMethodDef>,
        span: Span,
    },
    Fn(FnDecl),
    Import {
        namespace: Option<String>,
        items: Vec<String>,
        source: String,
        span: Span,
    },
    Export {
        decl: Box<Decl>,
        span: Span,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub declarations: Vec<Decl>,
}
