//! FFI symbol resolver and extern function validation.

use crate::abi::{CallingConvention, CPrimitive};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ExternFn {
    pub name: String,
    pub calling_convention: CallingConvention,
    pub params: Vec<(String, CPrimitive)>,
    pub return_type: CPrimitive,
    pub is_variadic: bool,
}

#[derive(Debug, Default)]
pub struct FfiResolver {
    pub extern_fns: HashMap<String, ExternFn>,
    pub native_links: Vec<String>,
}

impl FfiResolver {
    pub fn new() -> Self {
        let mut res = Self::default();
        res.register_stdlib_c_fns();
        res
    }

    fn register_stdlib_c_fns(&mut self) {
        self.extern_fns.insert(
            "malloc".to_string(),
            ExternFn {
                name: "malloc".to_string(),
                calling_convention: CallingConvention::C,
                params: vec![("size".to_string(), CPrimitive::SizeT)],
                return_type: CPrimitive::VoidPtr,
                is_variadic: false,
            },
        );

        self.extern_fns.insert(
            "free".to_string(),
            ExternFn {
                name: "free".to_string(),
                calling_convention: CallingConvention::C,
                params: vec![("ptr".to_string(), CPrimitive::VoidPtr)],
                return_type: CPrimitive::Int,
                is_variadic: false,
            },
        );

        self.extern_fns.insert(
            "printf".to_string(),
            ExternFn {
                name: "printf".to_string(),
                calling_convention: CallingConvention::C,
                params: vec![("format".to_string(), CPrimitive::VoidPtr)],
                return_type: CPrimitive::Int,
                is_variadic: true,
            },
        );
    }

    pub fn register_link(&mut self, lib_name: String) {
        if !self.native_links.contains(&lib_name) {
            self.native_links.push(lib_name);
        }
    }
}
