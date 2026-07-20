//! Standard Library compiler definitions and intrinsic mappings in Rust (`arca-stdlib`).

use std::collections::HashMap;

#[derive(Debug, Clone)]
pub struct StdSymbol {
    pub name: String,
    pub module: String,
    pub is_intrinsic: bool,
}

pub struct StdLibResolver {
    symbols: HashMap<String, Vec<StdSymbol>>,
}

impl StdLibResolver {
    pub fn new() -> Self {
        let mut symbols = HashMap::new();

        // Core module symbols (zero OS, zero allocator)
        let core_syms = vec![
            StdSymbol { name: "mem".into(), module: "core".into(), is_intrinsic: true },
            StdSymbol { name: "math".into(), module: "core".into(), is_intrinsic: true },
            StdSymbol { name: "hash".into(), module: "core".into(), is_intrinsic: false },
            StdSymbol { name: "encoding".into(), module: "core".into(), is_intrinsic: false },
        ];
        symbols.insert("core".into(), core_syms);

        // Std module symbols (OS, Network, I/O, Allocators)
        let std_alloc = vec![StdSymbol { name: "ArenaAllocator".into(), module: "std/alloc".into(), is_intrinsic: false }];
        let std_fs = vec![StdSymbol { name: "File".into(), module: "std/fs".into(), is_intrinsic: false }];
        let std_net = vec![StdSymbol { name: "TcpListener".into(), module: "std/net".into(), is_intrinsic: false }];
        let std_http = vec![StdSymbol { name: "Router".into(), module: "std/http".into(), is_intrinsic: false }];
        let std_json = vec![StdSymbol { name: "json".into(), module: "std/json".into(), is_intrinsic: true }];
        let std_os = vec![StdSymbol { name: "os".into(), module: "std/os".into(), is_intrinsic: false }];
        let std_process = vec![StdSymbol { name: "process".into(), module: "std/process".into(), is_intrinsic: false }];
        let std_time = vec![StdSymbol { name: "time".into(), module: "std/time".into(), is_intrinsic: false }];
        let std_crypto = vec![StdSymbol { name: "crypto".into(), module: "std/crypto".into(), is_intrinsic: false }];
        let std_compress = vec![StdSymbol { name: "compress".into(), module: "std/compress".into(), is_intrinsic: false }];
        let std_log = vec![StdSymbol { name: "log".into(), module: "std/log".into(), is_intrinsic: false }];

        symbols.insert("std/alloc".into(), std_alloc);
        symbols.insert("std/fs".into(), std_fs);
        symbols.insert("std/net".into(), std_net);
        symbols.insert("std/http".into(), std_http);
        symbols.insert("std/json".into(), std_json);
        symbols.insert("std/os".into(), std_os);
        symbols.insert("std/process".into(), std_process);
        symbols.insert("std/time".into(), std_time);
        symbols.insert("std/crypto".into(), std_crypto);
        symbols.insert("std/compress".into(), std_compress);
        symbols.insert("std/log".into(), std_log);

        Self { symbols }
    }

    pub fn is_stdlib_module(&self, path: &str) -> bool {
        self.symbols.contains_key(path)
    }

    pub fn get_module_symbols(&self, path: &str) -> Option<&[StdSymbol]> {
        self.symbols.get(path).map(|v| v.as_slice())
    }
}
