use arca_air::{AirBuilder, AirVerifier};
use arca_hir::Lowerer;
use arca_lexer::Lexer;
use arca_parser::Parser;

#[test]
fn test_air_builder_and_verifier() {
    let src = r#"
struct User {
    name: string,
    age: i32,

    fn get_age() -> i32 {
        return age
    }
}

fn main() {
    let user = User { name: "Lutfi", age: 25 }
    let val = 100
    let res = val + 50
}
"#;
    let lexer = Lexer::new(src);
    let mut parser = Parser::new(lexer);
    let program = parser.parse_program();
    assert!(parser.diagnostics().is_empty());

    let lowerer = Lowerer::new();
    let hir = lowerer.lower_program(&program);

    let mut builder = AirBuilder::new();
    let module = builder.build_module(&hir);

    assert!(module.functions.contains_key("main"));
    assert!(module.functions.contains_key("User.get_age"));

    let verifier_res = AirVerifier::verify_module(&module);
    assert!(verifier_res.is_ok());
}
