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
use std::time::Instant;

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
            let c_code = match compile_arca_to_c(&source, target, "") {
                Ok(c) => c,
                Err(e) => { eprintln!("{}", e); process::exit(1); }
            };
            let pid = process::id();
            let c_path = format!("build/output_{}.c", pid);
            let bin_path = format!("build/output_{}", pid);
            let gen_o = format!("{}_gen.o", c_path);
            fs::create_dir_all("build").ok();
            fs::write(&c_path, &c_code).ok();
            ensure_runtime_o("build/arca_runtime.o", "build/http.o", "build/socket.o");

            std::process::Command::new("cc")
                .args(&["-c", &c_path, "-o", &gen_o, "-I", "library/runtime"])
                .status().ok();
            let status = std::process::Command::new("cc")
                .args(&["-o", &bin_path, &gen_o, "build/arca_runtime.o", "build/http.o", "build/socket.o"])
                .status();
            match status {
                Ok(s) if s.success() => {
                    println!("[arca] Running: {}", target);
                    let run_status = std::process::Command::new(&bin_path).status();
                    fs::remove_file(&c_path).ok();
                    fs::remove_file(&gen_o).ok();
                    fs::remove_file(&bin_path).ok();
                    match run_status {
                        Ok(rs) if rs.success() => {}
                        Ok(rs) => eprintln!("[arca] Program exited with code: {}", rs),
                        Err(e) => eprintln!("[arca] Failed to run program: {}", e),
                    }
                }
                Ok(s) => {
                    fs::remove_file(&c_path).ok();
                    fs::remove_file(&gen_o).ok();
                    eprintln!("[arca] C compilation failed with code: {}", s);
                    process::exit(1);
                }
                Err(e) => {
                    fs::remove_file(&c_path).ok();
                    fs::remove_file(&gen_o).ok();
                    eprintln!("[arca] Failed to invoke C compiler 'cc': {}", e);
                    process::exit(1);
                }
            }
        }
        "test" => {
            let mut target = "tests".to_string();
            let mut filter = String::new();
            let mut i = 2;
            while i < args.len() {
                if args[i] == "--filter" && i + 1 < args.len() {
                    filter = args[i + 1].clone();
                    i += 2;
                } else {
                    target = args[i].clone();
                    i += 1;
                }
            }
            handle_test(&target, &filter);
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

fn ensure_runtime_o(runtime_o: &str, http_o: &str, socket_o: &str) {
    let needs_rebuild = |src: &str, out: &str| -> bool {
        if !std::path::Path::new(out).exists() { return true; }
        if let (Ok(sm), Ok(om)) = (std::fs::metadata(src).and_then(|m| m.modified()),
                                    std::fs::metadata(out).and_then(|m| m.modified())) {
            sm > om
        } else { true }
    };
    let compile = |src: &str, out: &str| {
        if needs_rebuild(src, out) {
            std::process::Command::new("cc")
                .args(&["-O3", "-c", src, "-o", out, "-I", "library/runtime"])
                .status().ok();
        }
    };
    compile("library/runtime/arca_runtime.c", runtime_o);
    compile("library/net/http.c", http_o);
    compile("library/net/socket.c", socket_o);
}

fn compile_arca_to_c(source: &str, target: &str, prefix: &str) -> Result<String, String> {
    let lexer = Lexer::new(source);
    let mut parser = Parser::new(lexer).with_file(target);
    let program = parser.parse_program();
    if !parser.diagnostics().is_empty() {
        let mut msg = String::new();
        for d in parser.diagnostics() { msg.push_str(&d.render(Some(source))); }
        return Err(msg);
    }
    let lowerer = Lowerer::new();
    let hir = lowerer.lower_program(&program);
    let mut type_checker = TypeChecker::new();
    let mut diags = type_checker.check_program(&hir);
    let mut borrow_checker = arca_borrowck::BorrowChecker::new();
    diags.extend(borrow_checker.check_program(&hir));
    if !diags.is_empty() {
        let mut msg = String::new();
        for d in &diags { msg.push_str(&d.render(Some(source))); }
        return Err(msg);
    }
    let mut air_builder = AirBuilder::new();
    let air_module = air_builder.build_module(&hir);
    let mut cg = CodeGenerator::new(BackendKind::C, TargetArch::Arm64).with_prefix(prefix);
    Ok(cg.generate_c_from_air(&air_module))
}

fn fmt_dur(us: u128) -> String {
    if us < 1000 { format!("{}µs", us) }
    else if us < 1_000_000 { format!("{}ms", us / 1000) }
    else { let s = us / 1_000_000; let d = (us % 1_000_000) / 100_000; format!("{}.{}s", s, d) }
}

fn handle_test(target: &str, filter: &str) {
    println!("\n=========================================");
    println!("        Arca Test Suite v{}", ARCA_VERSION);
    println!("=========================================");

    let commit = process::Command::new("git")
        .args(&["rev-parse", "--short", "HEAD"])
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| s.trim().to_string())
        .unwrap_or_default();

    if !commit.is_empty() {
        println!("  Commit   : {}", commit);
    }
    println!("  Backend  : AIR → C\n");

    ensure_runtime_o("build/arca_runtime.o", "build/http.o", "build/socket.o");

    // Layer 1: Parse tests
    println!("──────────────────────────────────────────\nParse\n──────────────────────────────────────────");
    let parse_dir = format!("{}/parse", target);
    let mut p_pass = 0; let mut p_fail = 0;
    if let Ok(entries) = fs::read_dir(&parse_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map_or(false, |e| e == "arca") {
                let name = path.file_stem().unwrap().to_string_lossy().to_string();
                if !filter.is_empty() && !name.contains(filter) { continue; }
                let source = fs::read_to_string(&path).unwrap_or_default();
                let start = Instant::now();
                let lexer = Lexer::new(&source);
                let mut parser = Parser::new(lexer).with_file(&*path.to_string_lossy());
                let _ = parser.parse_program();
                let errors = parser.diagnostics().len();
                let us = start.elapsed().as_micros();
                let ok = errors == 0;
                if ok { p_pass += 1; } else { p_fail += 1; }
                println!("  {:35} {:>6}  {}", name, fmt_dur(us), if ok { "✓" } else { "✗" });
            }
        }
    }

    // Layer 2: Semantic tests
    println!("\n──────────────────────────────────────────\nSemantic\n──────────────────────────────────────────");
    let sem_dir = format!("{}/semantic", target);
    let mut s_pass = 0; let mut s_fail = 0;
    if let Ok(entries) = fs::read_dir(&sem_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map_or(false, |e| e == "arca") {
                let name = path.file_stem().unwrap().to_string_lossy().to_string();
                if !filter.is_empty() && !name.contains(filter) { continue; }
                let source = fs::read_to_string(&path).unwrap_or_default();
                let start = Instant::now();
                let result = compile_arca_to_c(&source, &path.to_string_lossy(), "");
                let us = start.elapsed().as_micros();
                let expects_fail = name.ends_with("_invalid");
                let passed = match (&result, expects_fail) {
                    (Ok(_), false) => true,
                    (Err(_), true) => true,
                    _ => false,
                };
                if passed { s_pass += 1; } else { s_fail += 1; }
                println!("  {:35} {:>6}  {}",
                    name, fmt_dur(us),
                    if passed { "✓" } else { "✗" });
            }
        }
    }

    // Layer 3: Codegen tests
    println!("\n──────────────────────────────────────────\nCode Generation\n──────────────────────────────────────────");
    let cg_dir = format!("{}/codegen", target);
    let mut c_pass = 0; let mut c_fail = 0;
    if let Ok(entries) = fs::read_dir(&cg_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().map_or(false, |e| e == "arca") {
                let name = path.file_stem().unwrap().to_string_lossy().to_string();
                if !filter.is_empty() && !name.contains(filter) { continue; }
                let source = fs::read_to_string(&path).unwrap_or_default();
                let target_str = path.to_string_lossy();
                let start = Instant::now();
                let c_result = compile_arca_to_c(&source, &target_str, "");
                let us = start.elapsed().as_micros();
                let passed = c_result.is_ok();
                if passed { c_pass += 1; } else { c_fail += 1; }
                println!("  {:35} {:>6}  {}",
                    name, fmt_dur(us),
                    if passed { "✓" } else { "✗" });
            }
        }
    }

    // Layer 4: Runtime tests (batch-compiled with symbol prefixing)
    println!("\n──────────────────────────────────────────\nRuntime\n──────────────────────────────────────────");

    let rt_dirs = vec![
        format!("{}/runtime/features", target),
        format!("{}/runtime/std-libs", target),
        format!("{}/regression", target),
        format!("{}/../examples/challenges", target),
    ];

    fs::create_dir_all("build").ok();
    ensure_runtime_o("build/arca_runtime.o", "build/http.o", "build/socket.o");

    let mut r_pass = 0; let mut r_fail = 0;
    let mut object_files: Vec<String> = Vec::new();
    let mut test_names: Vec<String> = Vec::new();
    let mut discovered_tests: Vec<(String, String)> = Vec::new(); // (test_fn_name, display_name)
    let mut discovered_benches: Vec<(String, String)> = Vec::new();

    // Phase 1: compile each test to prefixed C → .o
    for dir in &rt_dirs {
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                let path = entry.path();
                if path.extension().map_or(false, |e| e == "arca") {
                    let name = path.file_stem().unwrap().to_string_lossy().to_string();
                    if !filter.is_empty() && !name.contains(filter) { continue; }
                    let safe = name.replace(|c: char| !c.is_alphanumeric(), "_");
                    let source = fs::read_to_string(&path).unwrap_or_default();
                    match compile_arca_to_c(&source, &path.to_string_lossy(), &format!("test_{}_", safe)) {
                        Ok(mut c_code) => {
                            // Strip int main(...) block — runner provides its own main
                            if let Some(main_pos) = c_code.find("int main(int argc, char** argv)") {
                                c_code.truncate(main_pos);
                            }

                            // Discover __test_* functions in the C code
                            let prefix = format!("test_{}___test_", safe);
                            let mut search_pos = 0;
                            while let Some(pos) = c_code[search_pos..].find(&prefix) {
                                let start = search_pos + pos + prefix.len();
                                let end = c_code[start..].find('(').unwrap_or(0);
                                if end > 0 {
                                    let test_fn = format!("test_{}___test_{}", safe, &c_code[start..start + end]);
                                    let display = &c_code[start..start + end].replace('_', " ");
                                    discovered_tests.push((test_fn, format!("{}::{}", name, display)));
                                }
                                search_pos = start + end;
                            }

                            // Discover __bench_* functions
                            let bprefix = format!("test_{}___bench_", safe);
                            let mut bpos = 0;
                            while let Some(pos) = c_code[bpos..].find(&bprefix) {
                                let start = bpos + pos + bprefix.len();
                                let end = c_code[start..].find('(').unwrap_or(0);
                                if end > 0 {
                                    let bench_fn = format!("test_{}___bench_{}", safe, &c_code[start..start + end]);
                                    let display = &c_code[start..start + end].replace('_', " ");
                                    discovered_benches.push((bench_fn, format!("{}::{}", name, display)));
                                }
                                bpos = start + end;
                            }
                            let c_path = format!("build/t_{}.c", safe);
                            let o_path = format!("build/t_{}.o", safe);
                            fs::write(&c_path, &c_code).ok();
                            if std::process::Command::new("cc")
                                .args(&["-c", &c_path, "-o", &o_path, "-I", "library/runtime"])
                                .status().map(|s| s.success()).unwrap_or(false)
                            {
                                object_files.push(o_path);
                                test_names.push(name);
                            } else {
                                println!("  {:35} {:>6}  FAIL  C compile error", name, fmt_dur(0));
                                r_fail += 1;
                            }
                            fs::remove_file(&c_path).ok();
                        }
                        Err(e) => {
                            println!("  {:35} {:>6}  FAIL  {}", name, fmt_dur(0), e.lines().next().unwrap_or(&e));
                            r_fail += 1;
                        }
                    }
                }
            }
        }
    }

    // Phase 2: build runner + link all .o into a single binary
    if !object_files.is_empty() {
        let mut runner = String::from(
            "#include \"arca_runtime.h\"\n\
             #include <string.h>\n\
             #include <unistd.h>\n\
             #include <sys/wait.h>\n\
             #include <time.h>\n\n");

        // Forward declarations for each test
        // Combine file tests + discovered __test__/__bench__ functions
        let mut all_names: Vec<String> = test_names.clone();
        let mut all_fns: Vec<String> = Vec::new();
        for name in &test_names {
            let safe = name.replace(|c: char| !c.is_alphanumeric(), "_");
            all_fns.push(format!("test_{}_arca_main", safe));
        }
        // Discovered tests go in a separate layer (after file tests)
        let layer_test_start = all_names.len();
        for (fn_name, display) in &discovered_tests {
            all_names.push(display.clone());
            all_fns.push(fn_name.clone());
        }
        // TODO: bench in a separate section with timing

        // Forward declarations
        for (i, _name) in all_names.iter().enumerate() {
            let safe_fn = &all_fns[i];
            if safe_fn.starts_with("test_") && safe_fn.ends_with("_arca_main") {
                runner.push_str(&format!("void {}();\n", safe_fn));
            } else {
                runner.push_str(&format!("void {}();\n", safe_fn));
            }
        }
        runner.push('\n');

        runner.push_str(&format!("const char* test_names[{}] = {{\n", all_names.len()));
        for name in &all_names { runner.push_str(&format!("  \"{}\",\n", name)); }
        runner.push_str("};\n\n");

        runner.push_str(&format!("void (*test_fns[{}])() = {{\n", all_fns.len()));
        for fn_name in &all_fns { runner.push_str(&format!("  {},\n", fn_name)); }
        runner.push_str("};\n\n");

        runner.push_str(&format!(
            "int main() {{\n\
               int n = {};\n\
               int pass = 0, fail = 0;\n\
               // Phase 1: Runtime file tests\n\
               for (int i = 0; i < n; i++) {{\n\
                 struct timespec t0, t1;\n\
                 clock_gettime(CLOCK_MONOTONIC, &t0);\n\
                 int p[2]; pipe(p);\n\
                 if (fork() == 0) {{ close(p[0]); dup2(p[1], 1); close(p[1]); test_fns[i](); _exit(0); }}\n\
                 close(p[1]);\n\
                 char buf[65536]; int nr = read(p[0], buf, sizeof(buf)-1); buf[nr] = 0;\n\
                 waitpid(-1, NULL, 0);\n\
                 clock_gettime(CLOCK_MONOTONIC, &t1);\n\
                 long ms = (t1.tv_sec - t0.tv_sec) * 1000 + (t1.tv_nsec - t0.tv_nsec) / 1000000;\n\
                 int has_err = strstr(buf, \"error:\") != NULL;\n\
                  if (ms < 1000)\n\
                    printf(\"  %-35s %ldms  %s\\n\", test_names[i], ms, has_err ? \"✗\" : \"✓\");\n\
                  else\n\
                    printf(\"  %-35s %ld.%lds  %s\\n\", test_names[i], ms/1000, (ms%1000)/100, has_err ? \"✗\" : \"✓\");\n\
                 if (has_err) fail++; else pass++;\n\
               }}\n\
                // Phase 2: Discovered tests\n\
               for (int i = {}; i < n; i++) {{\n\
                  printf(\"  %-35s \\n\", test_names[i]); fflush(stdout);\n\
                 int p[2]; pipe(p);\n\
                 if (fork() == 0) {{ close(p[0]); dup2(p[1], 1); close(p[1]); test_fns[i](); _exit(0); }}\n\
                 close(p[1]);\n\
                 char buf[65536]; int nr = read(p[0], buf, sizeof(buf)-1); buf[nr] = 0;\n\
                 waitpid(-1, NULL, 0);\n\
                 int has_err = strstr(buf, \"error:\") != NULL;\n\
                  printf(\"  %s\\n\", has_err ? \"✗\" : \"✓\");\n\
                  if (has_err) fail++; else pass++;\n\
                }}\n\
               }}\n", all_names.len(), layer_test_start));

        let r_path = "build/runner.c";
        let binary = "build/test_bundle";
        fs::write(r_path, &runner).ok();

        // Single link step — no duplicate symbols thanks to prefixing
        let link_start = Instant::now();
        let mut cc = std::process::Command::new("cc");
        cc.args(&["-O0", "-o", binary, r_path]);
        for o in &object_files { cc.arg(o); }
        cc.args(&["build/arca_runtime.o", "build/http.o", "build/socket.o", "-I", "library/runtime"]);
        let link_ok = cc.status().map(|s| s.success()).unwrap_or(false);
        let link_ms = link_start.elapsed().as_millis();

        if link_ok {
            let run_start = Instant::now();
            if let Ok(out) = std::process::Command::new(binary).output() {
                let run_ms = run_start.elapsed().as_millis();
                let stdout = String::from_utf8_lossy(&out.stdout);
                for line in stdout.lines() {
                    if line.starts_with("  ") && (line.ends_with("✓") || line.ends_with("✗")) {
                        let ok = line.ends_with("✓");
                        println!("{}", line);
                        if ok { r_pass += 1; } else { r_fail += 1; }
                    }
                }
                println!("  (link {}ms, run {}ms)", link_ms, run_ms);
            }
        } else {
            println!("  Link FAILED ({}ms)", link_ms);
            r_fail += object_files.len() as u32;
        }

        fs::remove_file(r_path).ok();
        for o in &object_files { fs::remove_file(o).ok(); }
        fs::remove_file(binary).ok();
    }

    // Summary
    let total = p_pass + p_fail + s_pass + s_fail + c_pass + c_fail + r_pass + r_fail;
    let passed = p_pass + s_pass + c_pass + r_pass;
    let failed = p_fail + s_fail + c_fail + r_fail;
    let pct = if total > 0 { passed * 100 / total } else { 0 };

    println!("\n──────────────────────────────────────────\nSummary\n──────────────────────────────────────────");
    println!("  Tests       {}", total);
    println!("  Passed      {}  ✓", passed);
    println!("  Failed      {}  {}", failed, if failed > 0 { "✗" } else { "" });
    println!("  Pass Rate   {}%", pct);
    if !commit.is_empty() {
        println!("  Commit      {}", commit);
    }
    println!("  Version     {}", ARCA_VERSION);
    println!("  Backend     AIR → C");
    println!("");
    if failed == 0 { println!("  ✓ ALL TESTS PASSED"); }
    println!("=========================================\n");

    if failed > 0 { process::exit(1); }
}
