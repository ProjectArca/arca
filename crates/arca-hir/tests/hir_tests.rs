use arca_hir::{HirExpr, Lowerer};
use arca_lexer::Lexer;
use arca_parser::Parser;

#[test]
fn test_hir_lowering_and_extend_merging() {
    let src = r#"
struct User {
    name: string,
    age: i32,
    fn get_name() -> string {
        return name
    }
}

extend User {
    fn new(name: string, age: i32) -> User {
        return User { name, age }
    }
}
"#;
    let lexer = Lexer::new(src);
    let mut parser = Parser::new(lexer);
    let program = parser.parse_program();

    let lowerer = Lowerer::new();
    let hir = lowerer.lower_program(&program);

    assert!(hir.structs.contains_key("User"));
    let user_struct = &hir.structs["User"];

    // Verify method merging (get_name from struct body AND new from extend block!)
    assert_eq!(user_struct.methods.len(), 2);
    assert!(user_struct.methods.contains_key("get_name"));
    assert!(user_struct.methods.contains_key("new"));

    // Verify desugaring of User { name, age } in new method
    let new_fn = &user_struct.methods["new"];
    if let Some(arca_hir::HirStmt::Return(Some(HirExpr::StructInit { fields, .. }))) = &new_fn.body.statements.get(0) {
        assert_eq!(fields.len(), 2);
        assert_eq!(fields[0].0, "name");
        assert_eq!(fields[0].1, HirExpr::VarRef("name".into()));
        assert_eq!(fields[1].0, "age");
        assert_eq!(fields[1].1, HirExpr::VarRef("age".into()));
    } else {
        panic!("Expected desugared StructInit in return statement");
    }
}

#[test]
fn test_hir_ownership_canonicalization() {
    let src = r#"
fn process_user(user: User) {
    let b = borrow(user)
    let m = move(user)
}
"#;
    let lexer = Lexer::new(src);
    let mut parser = Parser::new(lexer);
    let program = parser.parse_program();

    let lowerer = Lowerer::new();
    let hir = lowerer.lower_program(&program);

    assert!(hir.functions.contains_key("process_user"));
    let process_fn = &hir.functions["process_user"];

    if let Some(arca_hir::HirStmt::VarDecl { init: Some(HirExpr::Borrow(inner)), .. }) = &process_fn.body.statements.get(0) {
        assert_eq!(**inner, HirExpr::VarRef("user".into()));
    } else {
        panic!("Expected HirExpr::Borrow for borrow(user)");
    }

    if let Some(arca_hir::HirStmt::VarDecl { init: Some(HirExpr::Move(inner)), .. }) = &process_fn.body.statements.get(1) {
        assert_eq!(**inner, HirExpr::VarRef("user".into()));
    } else {
        panic!("Expected HirExpr::Move for move(user)");
    }
}
