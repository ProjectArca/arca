use arca_borrowck::BorrowChecker;
use arca_hir::Lowerer;
use arca_lexer::Lexer;
use arca_parser::Parser;

#[test]
fn test_valid_moves_and_borrows() {
    let src = r#"
struct User {
    name: string,
    age: i32,
}

fn main() {
    let user = User { name: "Lutfi", age: 25 }
    let b = borrow(user)
    let m = move(user)
}
"#;
    let lexer = Lexer::new(src);
    let mut parser = Parser::new(lexer);
    let program = parser.parse_program();

    let lowerer = Lowerer::new();
    let hir = lowerer.lower_program(&program);

    let mut checker = BorrowChecker::new();
    let diags = checker.check_program(&hir);

    assert!(diags.is_empty(), "Expected 0 borrow errors, got: {:#?}", diags);
}

#[test]
fn test_use_after_move_diagnostic() {
    let src = r#"
struct User {
    name: string,
}

fn main() {
    let user = User { name: "Lutfi" }
    let m1 = move(user)
    let m2 = move(user)
}
"#;
    let lexer = Lexer::new(src);
    let mut parser = Parser::new(lexer);
    let program = parser.parse_program();

    let lowerer = Lowerer::new();
    let hir = lowerer.lower_program(&program);

    let mut checker = BorrowChecker::new();
    let diags = checker.check_program(&hir);

    assert_eq!(diags.len(), 1);
    assert!(diags[0].message.contains("Use of moved value 'user'"));
}
