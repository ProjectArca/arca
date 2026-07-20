# Arca Programming Language

**Arca** is a modern, statically typed, native-compiled systems programming language designed for backend infrastructure, cloud services, high-performance network software, and AI tooling.

## Vision & Philosophy

Arca combines:
* **TypeScript-inspired syntax** for high developer productivity and ergonomics
* **Explicit memory management** (Zig-inspired allocators without garbage collection or Rust-style borrow checker complexity)
* **Zero-cost abstractions** and compile-time evaluation (`comptime`)
* **Go-style lightweight concurrency** (work-stealing scheduler)
* **First-class FFI** to directly import native C headers (`import c from "sqlite3.h"`)

For full details, read [philosophy.md](file:///Users/hy4-mac-002/hasdev/obsidian-docs/project-arca/rfc/philosophy.md) and [RFC-0000](file:///Users/hy4-mac-002/hasdev/obsidian-docs/project-arca/rfc/rfc-0000-Arca-Language-Vision-&-Philosophy.md.md).

## Compiler CLI Usage

```bash
# Build the arca CLI
cargo build --release

# Run arca commands
arca version
arca help
arca build program.arca
arca run program.arca
arca fmt src/
arca test tests/
```

## Repository Architecture

- `crates/arca-cli`: Command-line interface driver (`arca` binary)
- `crates/arca-lexer`: Tokenizer & UTF-8 source scanner
- `crates/arca-ast`: Abstract Syntax Tree definitions
- `crates/arca-diagnostics`: Compiler error formatting & diagnostics reporter

## License

Dual-licensed under MIT or Apache-2.0.
