use arca_hir::Lowerer;
use arca_lexer::Lexer;
use arca_parser::Parser;
use arca_typechecker::TypeChecker;

#[test]
fn test_valid_program_typecheck() {
    let src = r#"
struct User {
    name: string,
    age: i32,
    fn get_age() -> i32 {
        return age
    }
}

extend User {
    fn new(name: string, age: i32) -> User {
        return User { name, age }
    }
}

fn main() {
    let user = User.new("Lutfi", 25)
    let b = borrow(user)
    let m = move(user)
}
"#;
    let lexer = Lexer::new(src);
    let mut parser = Parser::new(lexer);
    let program = parser.parse_program();
    assert!(parser.diagnostics().is_empty());

    let lowerer = Lowerer::new();
    let hir = lowerer.lower_program(&program);

    let mut checker = TypeChecker::new();
    let diags = checker.check_program(&hir);

    assert!(diags.is_empty(), "Expected 0 type errors, found: {:#?}", diags);
}

#[test]
fn test_type_mismatch_diagnostics() {
    let src = r#"
fn test_err() {
    let age: i32 = "twenty-five"
}
"#;
    let lexer = Lexer::new(src);
    let mut parser = Parser::new(lexer);
    let program = parser.parse_program();

    let lowerer = Lowerer::new();
    let hir = lowerer.lower_program(&program);

    let mut checker = TypeChecker::new();
    let diags = checker.check_program(&hir);

    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.contains("Cannot assign type 'string' to variable 'age' of type 'i32'"));
}
