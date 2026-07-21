# Arca Programming Language

**Arca** is a modern, statically typed, native-compiled systems programming language designed for backend infrastructure, cloud services, high-performance network software, and AI tooling.

## Vision & Philosophy

Arca combines:
* **TypeScript-inspired syntax** for high developer productivity and ergonomics
* **Explicit memory management** (Zig-inspired allocators without garbage collection or Rust-style borrow checker complexity)
* **Zero-cost abstractions** and compile-time evaluation (`comptime`)
* **Go-style lightweight concurrency** (work-stealing scheduler)
* **First-class FFI** to directly import native C headers (`import c from "sqlite3.h"`)

## Compiler Pipeline

Source → Lexer → Parser → AST → HIR → **AIR** (stable IR) → C Backend → Clang/GCC

The AIR (Arca Intermediate Representation) is the compiler's contract — all language constructs lower to AIR, and all backends consume AIR.

## CLI Usage

```bash
# Build the arca CLI
cargo build --release

# Run arca commands
arca build program.arca   # Build to C
arca check program.arca   # Type check only
arca ast program.arca     # Show AST
arca hir program.arca     # Show HIR
arca air program.arca     # Show AIR (SSA IR)

# Compile generated C
cc -O3 -o program build/output.c -I library/runtime library/runtime/arca_runtime.c -lpthread
```

## Example Status

### Working (compile, link, run)
| Example | Description |
|---------|-------------|
| `http_server.arca` | Bun-style HTTP server with `serve({port, fetch(req){...}})` |
| `rest_api.arca` | CRUD API demo with JSON building, string concat, int-to-str |

### Compile clean (extern stubs, link unimplemented)
| Example | Description |
|---------|-------------|
| `allocators.arca` | Arena + Pool allocators |
| `collections.arca` | Array, Map, Deque generics |
| `concurrency.arca` | Channels, spawn, work-stealing |
| `fluent_chaining.arca` | Method chaining demo |

### Type check passes (C codegen structural gaps)
| Example | Description |
|---------|-------------|
| `capabilities_polymorphism.arca` | Trait dispatch, compile-time polymorphism |
| `demo.arca` | Full language demo |
| `destructuring.arca` | Struct destructuring |
| `ffi_native_c.arca` | Native C FFI calls |
| `std_crypto_compress.arca` | Crypto + compression std lib |
| `std_json_os_process.arca` | JSON + OS process std lib |
| `std_library.arca` | Standard library showcase |

### Pre-existing type errors (unimplemented language features)
| Example | Missing Feature |
|---------|----------------|
| `comptime_reflection.arca` | `sizeof`, compile-time reflection |
| `error_handling.arca` | `cfg` conditional compilation |
| `web_service.arca` | Error union types (`Err`, `WebError`) |

## Repository Architecture

- `crates/arca-cli/` — Command-line interface driver
- `crates/arca-lexer/` — Tokenizer & UTF-8 source scanner
- `crates/arca-parser/` — Pratt parser with struct literal method shorthand
- `crates/arca-ast/` — AST definitions
- `crates/arca-hir/` — High-level IR (name resolution, type desugaring)
- `crates/arca-air/` — SSA-based intermediate representation (stable IR)
- `crates/arca-typechecker/` — Type inference & checking
- `crates/arca-borrowck/` — Ownership & borrow checking
- `crates/arca-backend/` — C code generator (consumes AIR, emits C with `goto` CFG)
- `crates/arca-diagnostics/` — Error formatting & diagnostics
- `library/runtime/` — `arca_runtime.h` + `arca_runtime.c` (print, time, string ops)
- `library/net/` — Raw socket + HTTP server runtime (`http.c`)
- `library/std/` — Standard library (`.arca` type definitions)
- `benchmarks/` — Algorithm + web API benchmarks
- `examples/` — Language examples and CRUD REST API

## License

Dual-licensed under MIT or Apache-2.0.
