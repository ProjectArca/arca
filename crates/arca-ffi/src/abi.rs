//! C ABI Type definitions, calling conventions, and struct padding layout calculator.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CPrimitive {
    Char,
    Short,
    Int,
    Long,
    LongLong,
    Float,
    Double,
    VoidPtr,
    SizeT,
}

impl CPrimitive {
    pub fn size_and_align(&self) -> (usize, usize) {
        match self {
            CPrimitive::Char => (1, 1),
            CPrimitive::Short => (2, 2),
            CPrimitive::Int => (4, 4),
            CPrimitive::Long => (8, 8),
            CPrimitive::LongLong => (8, 8),
            CPrimitive::Float => (4, 4),
            CPrimitive::Double => (8, 8),
            CPrimitive::VoidPtr => (8, 8),
            CPrimitive::SizeT => (8, 8),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum CallingConvention {
    C,
    StdCall,
    FastCall,
    System,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CStructField {
    pub name: String,
    pub c_type: CPrimitive,
    pub offset: usize,
    pub size: usize,
    pub alignment: usize,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct CStructLayout {
    pub name: String,
    pub fields: Vec<CStructField>,
    pub total_size: usize,
    pub alignment: usize,
}

impl CStructLayout {
    pub fn compute<S: Into<String>>(name: S, fields_raw: Vec<(&str, CPrimitive)>) -> Self {
        let mut fields = Vec::new();
        let mut current_offset = 0;
        let mut max_alignment = 1;

        for (fname, ctype) in fields_raw {
            let (size, align) = ctype.size_and_align();
            if align > max_alignment {
                max_alignment = align;
            }

            // Align current offset
            let padding = (align - (current_offset % align)) % align;
            current_offset += padding;

            fields.push(CStructField {
                name: fname.to_string(),
                c_type: ctype,
                offset: current_offset,
                size,
                alignment: align,
            });

            current_offset += size;
        }

        // Align total size to max_alignment
        let tail_padding = (max_alignment - (current_offset % max_alignment)) % max_alignment;
        let total_size = current_offset + tail_padding;

        Self {
            name: name.into(),
            fields,
            total_size,
            alignment: max_alignment,
        }
    }
}
