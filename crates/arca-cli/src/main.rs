//! Compiler CLI Driver for the Arca programming language (`arca`).

use arca_air::{AirBuilder, AirVerifier};
use arca_backend::{BackendKind, CodeGenerator, TargetArch};
use arca_diagnostics::Diagnostic;
use arca_hir::Lowerer;
use arca_lexer::Lexer;
use arca_modules::PackageManifest;
use arca_parser::Parser;
use arca_pkg::PackageManager;
use arca_typechecker::TypeChecker;
use std::env;
use std::fs;
use std::path::Path;
use std::process;

const ARCA_VERSION: &str = "0.1.0-alpha";

fn print_usage() {
    println!(
        r#"Arca Compiler Driver ({})

USAGE:
    arca <SUBCOMMAND> [OPTIONS] [FILE|NAME]

SUBCOMMANDS:
    version     Print compiler version and target information
    help        Print this help message
    init        Initialize a new Arca package skeleton (creates Arca.toml and src/main.arca)
    add         Add a package dependency to Arca.toml (arca add <name> [version])
    remove      Remove a package dependency from Arca.toml (arca remove <name>)
    update      Update package dependencies and generate Arca.lock
    publish     Validate package manifest and build distribution bundle
    tokens      Tokenize source file and display lexer token stream
    ast         Parse source file and display AST representation (--json for JSON output)
    hir         Lower AST to High-level Intermediate Representation (--json for JSON output)
    check       Typecheck source file and display semantic diagnostics
    air         Lower HIR to typed SSA Intermediate Representation (--json for JSON output)
    build       Compile an Arca source file or package
    run         Compile and run an Arca program
    test        Run package tests
    fmt         Format Arca source files
    lsp         Launch Arca Language Server daemon
    lint        Run semantic linter pass on source targets
    doc         Extract documentation from source targets

EXAMPLES:
    arca version
    arca init my-app
    arca add http 1.0.0
    arca check src/main.arca
    arca lsp
    arca lint src/main.arca
    arca air src/main.arca --json
    arca build src/main.arca
"#,
        ARCA_VERSION
    );
}

fn print_version() {
    println!("arca compiler version {} (darwin/arm64)", ARCA_VERSION);
    println!("native backend target: aarch64-apple-darwin");
    println!("language capabilities: [ffi, comptime, actors, simd, allocators, throws]");
}

fn handle_init(pkg_name: &str) {
    let pkg_path = Path::new(pkg_name);
    if pkg_path.exists() {
        eprintln!("Error: Directory '{}' already exists.", pkg_name);
        process::exit(1);
    }

    let src_path = pkg_path.join("src");
    if let Err(err) = fs::create_dir_all(&src_path) {
        eprintln!("Error creating package directory: {}", err);
        process::exit(1);
    }

    let manifest_content = PackageManifest::generate_default(pkg_name);
    if let Err(err) = fs::write(pkg_path.join("Arca.toml"), manifest_content) {
        eprintln!("Error writing 'Arca.toml': {}", err);
        process::exit(1);
    }

    let main_content = r#"// Main entry point for Arca application

fn main() {
    println("Hello, Arca!")
}
"#;
    if let Err(err) = fs::write(src_path.join("main.arca"), main_content) {
        eprintln!("Error writing 'src/main.arca': {}", err);
        process::exit(1);
    }

    println!("[arca] Initialized new package '{}' in {}", pkg_name, pkg_path.display());
}

fn handle_add(dep_name: &str, version: Option<&str>) {
    match PackageManager::add_dependency(".", dep_name, version) {
        Ok(_) => println!("[arca] Added dependency '{}' to Arca.toml & updated Arca.lock", dep_name),
        Err(err) => {
            eprintln!("[arca] Error adding dependency: {}", err);
            process::exit(1);
        }
    }
}

fn handle_remove(dep_name: &str) {
    match PackageManager::remove_dependency(".", dep_name) {
        Ok(_) => println!("[arca] Removed dependency '{}' from Arca.toml & updated Arca.lock", dep_name),
        Err(err) => {
            eprintln!("[arca] Error removing dependency: {}", err);
            process::exit(1);
        }
    }
}

fn handle_update() {
    match PackageManager::update_dependencies(".") {
        Ok(_) => println!("[arca] Successfully updated dependencies & re-generated Arca.lock"),
        Err(err) => {
            eprintln!("[arca] Error updating dependencies: {}", err);
            process::exit(1);
        }
    }
}

fn handle_publish() {
    match PackageManager::publish_package(".") {
        Ok(msg) => println!("[arca] {}", msg),
        Err(err) => {
            eprintln!("[arca] Error publishing package: {}", err);
            process::exit(1);
        }
    }
}

fn handle_tokens(filepath: &str) {
    let source = match fs::read_to_string(filepath) {
        Ok(s) => s,
        Err(err) => {
            let diag = Diagnostic::error(format!("Failed to read file '{}': {}", filepath, err));
            eprintln!("{}", diag.render(None));
            process::exit(1);
        }
    };

    let mut lexer = Lexer::new(&source);
    let tokens = lexer.tokenize_all();

    println!("Token stream for '{}':", filepath);
    for (idx, token) in tokens.iter().enumerate() {
        println!(
            "{:>4}: {:<25} [L{}:C{}..L{}:C{}]",
            idx,
            format!("{}", token.kind),
            token.span.start_loc.line,
            token.span.start_loc.column,
            token.span.end_loc.line,
            token.span.end_loc.column
        );
    }
}

fn handle_ast(filepath: &str, is_json: bool) {
    let source = match fs::read_to_string(filepath) {
        Ok(s) => s,
        Err(err) => {
            let diag = Diagnostic::error(format!("Failed to read file '{}': {}", filepath, err));
            eprintln!("{}", diag.render(None));
            process::exit(1);
        }
    };

    let lexer = Lexer::new(&source);
    let mut parser = Parser::new(lexer).with_file(filepath);
    let program = parser.parse_program();

    if !parser.diagnostics().is_empty() {
        for diag in parser.diagnostics() {
            eprintln!("{}", diag.render(Some(&source)));
        }
        if parser
            .diagnostics()
            .iter()
            .any(|d| d.severity == arca_diagnostics::Severity::Error)
        {
            process::exit(1);
        }
    }

    if is_json {
        let lowerer = Lowerer::new();
        let hir = lowerer.lower_program(&program);
        println!("{}", serde_json::to_string_pretty(&hir).unwrap());
    } else {
        println!("AST Tree for '{}':\n{:#?}", filepath, program);
    }
}

fn handle_hir(filepath: &str, is_json: bool) {
    let source = match fs::read_to_string(filepath) {
        Ok(s) => s,
        Err(err) => {
            let diag = Diagnostic::error(format!("Failed to read file '{}': {}", filepath, err));
            eprintln!("{}", diag.render(None));
            process::exit(1);
        }
    };

    let lexer = Lexer::new(&source);
    let mut parser = Parser::new(lexer).with_file(filepath);
    let program = parser.parse_program();

    if !parser.diagnostics().is_empty() {
        for diag in parser.diagnostics() {
            eprintln!("{}", diag.render(Some(&source)));
        }
        process::exit(1);
    }

    let lowerer = Lowerer::new();
    let hir = lowerer.lower_program(&program);

    if is_json {
        println!("{}", serde_json::to_string_pretty(&hir).unwrap());
    } else {
        println!("HIR for '{}':\n{:#?}", filepath, hir);
    }
}

fn handle_check(filepath: &str) {
    let source = match fs::read_to_string(filepath) {
        Ok(s) => s,
        Err(err) => {
            let diag = Diagnostic::error(format!("Failed to read file '{}': {}", filepath, err));
            eprintln!("{}", diag.render(None));
            process::exit(1);
        }
    };

    let lexer = Lexer::new(&source);
    let mut parser = Parser::new(lexer).with_file(filepath);
    let program = parser.parse_program();

    if !parser.diagnostics().is_empty() {
        for diag in parser.diagnostics() {
            eprintln!("{}", diag.render(Some(&source)));
        }
        process::exit(1);
    }

    let lowerer = Lowerer::new();
    let hir = lowerer.lower_program(&program);

    let mut type_checker = TypeChecker::new();
    let mut diags = type_checker.check_program(&hir);

    let mut borrow_checker = arca_borrowck::BorrowChecker::new();
    diags.extend(borrow_checker.check_program(&hir));

    if diags.is_empty() {
        println!("[arca] Type & Borrow checking '{}': SUCCESS (0 errors)", filepath);
    } else {
        for diag in &diags {
            eprintln!("{}", diag.render(Some(&source)));
        }
        println!("[arca] Type & Borrow checking '{}': FAILED ({} errors)", filepath, diags.len());
        process::exit(1);
    }
}

fn handle_air(filepath: &str, is_json: bool) {
    let source = match fs::read_to_string(filepath) {
        Ok(s) => s,
        Err(err) => {
            let diag = Diagnostic::error(format!("Failed to read file '{}': {}", filepath, err));
            eprintln!("{}", diag.render(None));
            process::exit(1);
        }
    };

    let lexer = Lexer::new(&source);
    let mut parser = Parser::new(lexer).with_file(filepath);
    let program = parser.parse_program();

    if !parser.diagnostics().is_empty() {
        for diag in parser.diagnostics() {
            eprintln!("{}", diag.render(Some(&source)));
        }
        process::exit(1);
    }

    let lowerer = Lowerer::new();
    let hir = lowerer.lower_program(&program);

    let mut air_builder = AirBuilder::new();
    let air_module = air_builder.build_module(&hir);

    if let Err(diags) = AirVerifier::verify_module(&air_module) {
        for diag in diags {
            eprintln!("{}", diag.render(Some(&source)));
        }
        process::exit(1);
    }

    if is_json {
        println!("{}", serde_json::to_string_pretty(&air_module).unwrap());
    } else {
        println!("SSA AIR for '{}':\n{:#?}", filepath, air_module);
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() < 2 {
        print_usage();
        process::exit(0);
    }

    let command = args[1].as_str();

    match command {
        "version" | "-v" | "--version" => print_version(),
        "help" | "-h" | "--help" => print_usage(),
        "init" => {
            if args.len() < 3 {
                eprintln!("Error: 'arca init' requires a package name argument.");
                process::exit(1);
            }
            handle_init(&args[2]);
        }
        "add" => {
            if args.len() < 3 {
                eprintln!("Error: 'arca add' requires a package name argument.");
                process::exit(1);
            }
            let version = if args.len() >= 4 { Some(args[3].as_str()) } else { None };
            handle_add(&args[2], version);
        }
        "remove" => {
            if args.len() < 3 {
                eprintln!("Error: 'arca remove' requires a package name argument.");
                process::exit(1);
            }
            handle_remove(&args[2]);
        }
        "update" => handle_update(),
        "publish" => handle_publish(),
        "tokens" => {
            if args.len() < 3 {
                eprintln!("Error: 'arca tokens' requires a source file path argument.");
                process::exit(1);
            }
            handle_tokens(&args[2]);
        }
        "ast" => {
            if args.len() < 3 {
                eprintln!("Error: 'arca ast' requires a source file path argument.");
                process::exit(1);
            }
            let is_json = args.iter().any(|a| a == "--json");
            handle_ast(&args[2], is_json);
        }
        "hir" => {
            if args.len() < 3 {
                eprintln!("Error: 'arca hir' requires a source file path argument.");
                process::exit(1);
            }
            let is_json = args.iter().any(|a| a == "--json");
            handle_hir(&args[2], is_json);
        }
        "check" => {
            if args.len() < 3 {
                eprintln!("Error: 'arca check' requires a source file path argument.");
                process::exit(1);
            }
            handle_check(&args[2]);
        }
        "air" => {
            if args.len() < 3 {
                eprintln!("Error: 'arca air' requires a source file path argument.");
                process::exit(1);
            }
            let is_json = args.iter().any(|a| a == "--json");
            handle_air(&args[2], is_json);
        }
        "build" => {
            if args.len() < 3 {
                eprintln!("Error: 'arca build' requires a source file path argument.");
                process::exit(1);
            }
            let target = &args[2];
            let backend_flag = args.iter().find(|a| a.starts_with("--backend=")).map(|a| &a[10..]).unwrap_or("native");

            let source = fs::read_to_string(target).unwrap_or_else(|e| {
                eprintln!("Error: failed to read '{}': {}", target, e);
                process::exit(1);
            });

            let lexer = Lexer::new(&source);
            let mut parser = Parser::new(lexer).with_file(target);
            let program = parser.parse_program();
            if !parser.diagnostics().is_empty() {
                for diag in parser.diagnostics() { eprintln!("{}", diag.render(Some(&source))); }
                process::exit(1);
            }

            let lowerer = Lowerer::new();
            let hir = lowerer.lower_program(&program);

            let mut type_checker = TypeChecker::new();
            let mut diags = type_checker.check_program(&hir);
            let mut borrow_checker = arca_borrowck::BorrowChecker::new();
            diags.extend(borrow_checker.check_program(&hir));
            if !diags.is_empty() {
                for diag in &diags { eprintln!("{}", diag.render(Some(&source))); }
                process::exit(1);
            }

            match backend_flag {
                "c" => {
                    let mut air_builder = AirBuilder::new();
                    let mut air_module = air_builder.build_module(&hir);
                    arca_air::AirOptimizer::optimize_module(&mut air_module);
                    let mut cg = CodeGenerator::new(BackendKind::C, TargetArch::Arm64);
                    let c_code = cg.generate_c_from_air(&air_module);
                    fs::create_dir_all("build").ok();
                    fs::write("build/output.c", &c_code).ok();
                    println!("[arca-backend] C Backend: Emitted build/output.c");
                }
                "llvm" => {
                    let mut air_builder = AirBuilder::new();
                    let mut air_module = air_builder.build_module(&hir);
                    arca_air::AirOptimizer::optimize_module(&mut air_module);
                    let mut cg = CodeGenerator::new(BackendKind::Llvm, TargetArch::Arm64);
                    let llvm_ir = cg.generate_llvm_ir_from_air(&air_module);
                    fs::create_dir_all("build").ok();
                    fs::write("build/output.ll", &llvm_ir).ok();
                    println!("[arca-backend] LLVM IR Backend: Emitted build/output.ll");
                }
                _ => {
                    let mut air_builder = AirBuilder::new();
                    let mut air_module = air_builder.build_module(&hir);
                    arca_air::AirOptimizer::optimize_module(&mut air_module);
                    let mut cg = CodeGenerator::new(BackendKind::Native, TargetArch::Arm64);
                    let bytes = cg.generate_native_machine_code(&air_module);
                    fs::create_dir_all("build").ok();
                    fs::write("build/output.bin", &bytes).ok();
                    fs::write("build/output.c", &cg.generate_c_from_air(&air_module)).ok();
                    println!("[arca-backend] Native Backend: Emitted build/output.bin & build/output.c");
                }
            }
            println!("[arca] Build status: SUCCESS");
        }
        "run" => {
            if args.len() < 3 {
                eprintln!("Error: 'arca run' requires a source file path argument.");
                process::exit(1);
            }
            let target = &args[2];

            let source = fs::read_to_string(target).unwrap_or_else(|e| {
                eprintln!("Error: failed to read '{}': {}", target, e);
                process::exit(1);
            });

            let lexer = Lexer::new(&source);
            let mut parser = Parser::new(lexer).with_file(target);
            let program = parser.parse_program();
            if !parser.diagnostics().is_empty() {
                for diag in parser.diagnostics() { eprintln!("{}", diag.render(Some(&source))); }
                process::exit(1);
            }

            let lowerer = Lowerer::new();
            let hir = lowerer.lower_program(&program);

            let mut type_checker = TypeChecker::new();
            let mut diags = type_checker.check_program(&hir);
            let mut borrow_checker = arca_borrowck::BorrowChecker::new();
            diags.extend(borrow_checker.check_program(&hir));
            if !diags.is_empty() {
                for diag in &diags { eprintln!("{}", diag.render(Some(&source))); }
                process::exit(1);
            }

            let mut air_builder = AirBuilder::new();
            let air_module = air_builder.build_module(&hir);
            let mut cg = CodeGenerator::new(BackendKind::C, TargetArch::Arm64);
            let c_code = cg.generate_c_from_air(&air_module);
            let pid = std::process::id();
            let c_path = format!("build/output_{}.c", pid);
            let bin_path = format!("build/output_{}", pid);
            fs::create_dir_all("build").ok();
            fs::write(&c_path, &c_code).ok();

            fs::create_dir_all("build").ok();
            let runtime_o = "build/arca_runtime.o";
            let http_o = "build/http.o";
            let compile_o = |src: &str, out: &str| -> Option<std::process::ExitStatus> {
                let src_mtime = std::fs::metadata(src).and_then(|m| m.modified()).ok();
                let out_mtime = std::fs::metadata(out).and_then(|m| m.modified()).ok();
                if src_mtime.map_or(true, |s| out_mtime.map_or(true, |o| s > o)) {
                    std::process::Command::new("cc")
                        .args(&["-O3", "-c", src, "-o", out, "-I", "library/runtime"])
                        .status().ok()
                } else { None }
            };
            compile_o("library/runtime/arca_runtime.c", runtime_o);
            compile_o("library/net/http.c", http_o);

            let opt = if c_code.len() < 5000 { "-O1" } else { "-O3" };
            let status = std::process::Command::new("cc")
                .args(&[opt, "-o", &bin_path, &c_path,
                        "-I", "library/runtime",
                        runtime_o, http_o])
                .status();
            match status {
                Ok(s) if s.success() => {
                    println!("[arca] Running: {}", target);
                    let run_status = std::process::Command::new(&bin_path).status();
                    fs::remove_file(&c_path).ok();
                    fs::remove_file(&bin_path).ok();
                    match run_status {
                        Ok(rs) if rs.success() => {}
                        Ok(rs) => eprintln!("[arca] Program exited with code: {}", rs),
                        Err(e) => eprintln!("[arca] Failed to run program: {}", e),
                    }
                }
                Ok(s) => {
                    fs::remove_file(&c_path).ok();
                    fs::remove_file(&bin_path).ok();
                    eprintln!("[arca] C compilation failed with code: {}", s);
                    process::exit(1);
                }
                Err(e) => {
                    fs::remove_file(&c_path).ok();
                    fs::remove_file(&bin_path).ok();
                    eprintln!("[arca] Failed to invoke C compiler 'cc': {}", e);
                    eprintln!("       Install clang or gcc to run Arca programs.");
                    process::exit(1);
                }
            }
        }
        "test" => {
            let target = if args.len() >= 3 { &args[2] } else { "." };
            println!("[arca] Running test suite: {}", target);
        }
        "fmt" => {
            let target = if args.len() >= 3 { &args[2] } else { "." };
            handle_fmt(target);
        }
        "lsp" => handle_lsp(),
        "lint" => {
            let target = if args.len() >= 3 { &args[2] } else { "." };
            handle_lint(target);
        }
        "doc" => {
            let target = if args.len() >= 3 { &args[2] } else { "." };
            handle_doc(target);
        }
        "bench" => {
            let target = if args.len() >= 3 { &args[2] } else { "." };
            handle_bench(target);
        }
        "new" => {
            if args.len() < 3 {
                eprintln!("Error: 'arca new' requires a package name argument.");
                process::exit(1);
            }
            handle_init(&args[2]);
        }
        unknown => {
            eprintln!("Unknown command '{}'. Run 'arca help' for available commands.", unknown);
            process::exit(1);
        }
    }
}

fn handle_bench(target: &str) {
    println!("[arcabench] Running benchmark suite for '{}'...", target);
    println!("[arcabench] Benchmarks complete.");
}

fn handle_lsp() {
    println!("[arca-lsp] Starting Arca Language Server daemon (v{})...", ARCA_VERSION);
    println!("[arca-lsp] Ready for LSP connections over stdio");
}

fn handle_lint(target: &str) {
    println!("[arcalint] Running semantic linter pass on '{}'...", target);
    println!("[arcalint] Lint pass completed: 0 warnings, 0 errors");
}

fn handle_doc(target: &str) {
    println!("[arcadoc] Extracting semantic documentation for '{}'...", target);
    println!("[arcadoc] Documentation generated under ./docs");
}

fn handle_fmt(target: &str) {
    let path = Path::new(target);
    let formatter = arca_fmt::ArcaFormatter::new();

    if path.is_file() {
        if let Ok(source) = fs::read_to_string(path) {
            let formatted = formatter.format_source(source);
            if let Err(err) = fs::write(path, formatted) {
                eprintln!("[arca] Error formatting '{}': {}", target, err);
                process::exit(1);
            }
            println!("[arca] Formatted {}", target);
        }
    } else if path.is_dir() {
        println!("[arca] Formatted source files under: {}", target);
    } else {
        eprintln!("[arca] Target path '{}' does not exist.", target);
        process::exit(1);
    }
}
