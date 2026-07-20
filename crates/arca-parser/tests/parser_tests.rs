use arca_ast::*;
use arca_lexer::Lexer;
use arca_parser::Parser;

#[test]
fn test_parse_struct_and_extend() {
    let src = r#"
struct User {
    name: string,
    age: i32,
    fn inline_method() -> i32 {
        return 42
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

    assert!(parser.diagnostics().is_empty());
    assert_eq!(program.declarations.len(), 2);

    match &program.declarations[0] {
        Decl::Struct { name, fields, methods, .. } => {
            assert_eq!(name, "User");
            assert_eq!(fields.len(), 2);
            assert_eq!(methods.len(), 1);
            assert_eq!(methods[0].name, "inline_method");
        }
        _ => panic!("Expected Struct declaration"),
    }

    match &program.declarations[1] {
        Decl::Extend { target_name, methods, .. } => {
            assert_eq!(target_name, "User");
            assert_eq!(methods.len(), 1);
            assert_eq!(methods[0].name, "new");
        }
        _ => panic!("Expected Extend declaration"),
    }
}

#[test]
fn test_parse_expressions_and_intrinsics() {
    let src = r#"
fn test_fn() {
    let b = borrow(user)
    let m = move(user)
    let intr = @borrow(user)
    const table = comptime {
        generateTable()
    }
    spawn {
        process()
    }
}
"#;
    let lexer = Lexer::new(src);
    let mut parser = Parser::new(lexer);
    let program = parser.parse_program();

    assert!(parser.diagnostics().is_empty());
    assert_eq!(program.declarations.len(), 1);

    if let Decl::Fn(fndecl) = &program.declarations[0] {
        assert_eq!(fndecl.name, "test_fn");
        println!("Parsed statements: {:#?}", fndecl.body.statements);
        assert!(!fndecl.body.statements.is_empty());
    } else {
        panic!("Expected Fn declaration");
    }
}

#[test]
fn test_parse_expression_oriented_if() {
    let src = r#"
fn check(ok: bool) -> i32 {
    let val = if ok { 100 } else { 200 }
    return val
}
"#;
    let lexer = Lexer::new(src);
    let mut parser = Parser::new(lexer);
    let program = parser.parse_program();

    assert!(parser.diagnostics().is_empty());
    assert_eq!(program.declarations.len(), 1);
}
