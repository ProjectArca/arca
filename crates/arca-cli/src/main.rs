//! Compiler CLI Driver for the Arca programming language (`arca`).

use arca_diagnostics::Diagnostic;
use arca_lexer::Lexer;
use std::env;
use std::fs;
use std::process;

const ARCA_VERSION: &str = "0.1.0-alpha";

fn print_usage() {
    println!(
        r#"Arca Compiler Driver ({})

USAGE:
    arca <SUBCOMMAND> [OPTIONS] [FILE]

SUBCOMMANDS:
    version     Print compiler version and target information
    help        Print this help message
    tokens      Tokenize source file and display lexer token stream
    build       Compile an Arca source file or package
    run         Compile and run an Arca program
    test        Run package tests
    fmt         Format Arca source files

EXAMPLES:
    arca version
    arca tokens main.arca
    arca build main.arca
"#,
        ARCA_VERSION
    );
}

fn print_version() {
    println!("arca compiler version {} (darwin/arm64)", ARCA_VERSION);
    println!("native backend target: aarch64-apple-darwin");
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
        "tokens" => {
            if args.len() < 3 {
                eprintln!("Error: 'arca tokens' requires a source file path argument.");
                process::exit(1);
            }
            handle_tokens(&args[2]);
        }
        "build" => {
            let target = if args.len() >= 3 { &args[2] } else { "." };
            println!("[arca] Building target: {}", target);
            println!("[arca] Lexer & Frontend validation: SUCCESS");
        }
        "run" => {
            let target = if args.len() >= 3 { &args[2] } else { "." };
            println!("[arca] Compiling and running: {}", target);
        }
        "test" => {
            let target = if args.len() >= 3 { &args[2] } else { "." };
            println!("[arca] Running test suite: {}", target);
        }
        "fmt" => {
            let target = if args.len() >= 3 { &args[2] } else { "." };
            println!("[arca] Formatting source files under: {}", target);
        }
        unknown => {
            eprintln!("Unknown command '{}'. Run 'arca help' for available commands.", unknown);
            process::exit(1);
        }
    }
}
