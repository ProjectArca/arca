//! Fast String Interning for Symbol management in Arca.

use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Symbol(pub u32);

impl Symbol {
    pub const EMPTY: Symbol = Symbol(0);
}

impl fmt::Display for Symbol {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "Sym({})", self.0)
    }
}

#[derive(Debug, Default)]
pub struct SymbolTable {
    map: HashMap<String, Symbol>,
    strings: Vec<String>,
}

impl SymbolTable {
    pub fn new() -> Self {
        let mut table = Self {
            map: HashMap::new(),
            strings: Vec::new(),
        };
        // Reserve empty symbol at index 0
        table.intern("");
        table
    }

    pub fn intern<S: AsRef<str>>(&mut self, s: S) -> Symbol {
        let str_ref = s.as_ref();
        if let Some(&sym) = self.map.get(str_ref) {
            return sym;
        }

        let sym = Symbol(self.strings.len() as u32);
        let owned = str_ref.to_string();
        self.strings.push(owned.clone());
        self.map.insert(owned, sym);
        sym
    }

    pub fn resolve(&self, sym: Symbol) -> Option<&str> {
        self.strings.get(sym.0 as usize).map(|s| s.as_str())
    }
}
