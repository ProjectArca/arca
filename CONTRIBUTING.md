# Contributing to Arca

Thank you for contributing to the Arca programming language!

## Task Categorization & Task Prefixes

Every compiler contribution should be prefixed according to its target subsystem:

| Stage | Prefix | Example |
| ----- | ------ | ------- |
| Frontend | `FE-` | `FE-021 Implement string literal lexer` |
| Semantic | `SE-` | `SE-034 Generic type inference` |
| AIR | `IR-` | `IR-012 SSA builder` |
| Backend | `BE-` | `BE-041 ARM64 instruction selector` |
| Runtime | `RT-` | `RT-008 Task scheduler` |
| Standard Library | `STD-` | `STD-102 JSON parser` |
| Tooling | `TL-` | `TL-014 LSP hover provider` |
| Documentation | `DOC-` | `DOC-009 Ownership chapter` |

## Development Workflow

1. Format code before committing: `cargo fmt`
2. Run linters: `cargo clippy --workspace`
3. Execute all unit and golden tests: `cargo test --workspace`
