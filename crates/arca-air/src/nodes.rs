//! SSA IR nodes, Basic Blocks, and Control Flow Graph (CFG) structures for Arca AIR.

use arca_ast::BinaryOp;
use arca_typechecker::Type;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct RegisterId(pub u32);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BlockId(pub u32);

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AirValue {
    ConstInt(i64),
    ConstFloat(f64),
    ConstBool(bool),
    ConstString(String),
    Register(RegisterId),
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AirInstruction {
    Alloca {
        target: RegisterId,
        ty: Type,
    },
    Load {
        target: RegisterId,
        ptr: RegisterId,
        ty: Type,
    },
    Store {
        ptr: RegisterId,
        val: AirValue,
    },
    Binary {
        target: RegisterId,
        op: BinaryOp,
        left: AirValue,
        right: AirValue,
    },
    Call {
        target: Option<RegisterId>,
        fn_name: String,
        args: Vec<AirValue>,
    },
    StructInit {
        target: RegisterId,
        struct_name: String,
        fields: Vec<(String, AirValue)>,
    },
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum AirTerminator {
    Br(BlockId),
    CondBr {
        cond: AirValue,
        then_block: BlockId,
        else_block: BlockId,
    },
    Ret(Option<AirValue>),
    Unreachable,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct BasicBlock {
    pub id: BlockId,
    pub instructions: Vec<AirInstruction>,
    pub terminator: AirTerminator,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AirFunction {
    pub name: String,
    pub params: Vec<(String, Type)>,
    pub return_type: Type,
    pub blocks: Vec<BasicBlock>,
    pub entry_block: BlockId,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AirModule {
    pub name: String,
    pub functions: HashMap<String, AirFunction>,
}

impl AirModule {
    pub fn new(name: String) -> Self {
        Self {
            name,
            functions: HashMap::new(),
        }
    }
}
