use arca_lexer::{Lexer, TokenKind};

#[test]
fn test_keywords_and_identifiers() {
    let src = "let user = User.new() extend User borrow(user) move(user) comptime { spawn { process() } }";
    let mut lexer = Lexer::new(src);
    let tokens = lexer.tokenize_all();

    assert_eq!(tokens[0].kind, TokenKind::Let);
    assert_eq!(tokens[1].kind, TokenKind::Identifier("user".into()));
    assert_eq!(tokens[2].kind, TokenKind::Assign);
    assert_eq!(tokens[3].kind, TokenKind::Identifier("User".into()));
    assert_eq!(tokens[4].kind, TokenKind::Dot);
    assert_eq!(tokens[5].kind, TokenKind::Identifier("new".into()));
    assert_eq!(tokens[6].kind, TokenKind::OpenParen);
    assert_eq!(tokens[7].kind, TokenKind::CloseParen);
    assert_eq!(tokens[8].kind, TokenKind::Extend);
    assert_eq!(tokens[9].kind, TokenKind::Identifier("User".into()));
}

#[test]
fn test_numbers_and_strings() {
    let src = "42 0x1F 0b1010 3.14 \"Hello, Arca\\n\" 'x'";
    let mut lexer = Lexer::new(src);
    let tokens = lexer.tokenize_all();

    assert_eq!(tokens[0].kind, TokenKind::IntLiteral(42));
    assert_eq!(tokens[1].kind, TokenKind::IntLiteral(31));
    assert_eq!(tokens[2].kind, TokenKind::IntLiteral(10));
    assert_eq!(tokens[3].kind, TokenKind::FloatLiteral(3.14));
    assert_eq!(tokens[4].kind, TokenKind::StringLiteral("Hello, Arca\n".into()));
    assert_eq!(tokens[5].kind, TokenKind::CharLiteral('x'));
    assert_eq!(tokens[6].kind, TokenKind::Eof);
}

#[test]
fn test_operators_and_delimiters() {
    let src = "fn add(a: i32, b: i32) -> i32 { return a + b; } ?. ?? => ::";
    let mut lexer = Lexer::new(src);
    let tokens = lexer.tokenize_all();

    assert_eq!(tokens[0].kind, TokenKind::Fn);
    assert_eq!(tokens[1].kind, TokenKind::Identifier("add".into()));
    assert_eq!(tokens[2].kind, TokenKind::OpenParen);
    assert_eq!(tokens[3].kind, TokenKind::Identifier("a".into()));
    assert_eq!(tokens[4].kind, TokenKind::Colon);
    assert_eq!(tokens[5].kind, TokenKind::Identifier("i32".into()));
    assert_eq!(tokens[6].kind, TokenKind::Comma);
    assert_eq!(tokens[7].kind, TokenKind::Identifier("b".into()));
    assert_eq!(tokens[8].kind, TokenKind::Colon);
    assert_eq!(tokens[9].kind, TokenKind::Identifier("i32".into()));
    assert_eq!(tokens[10].kind, TokenKind::CloseParen);
    assert_eq!(tokens[11].kind, TokenKind::Arrow);
    assert_eq!(tokens[12].kind, TokenKind::Identifier("i32".into()));
    assert_eq!(tokens[13].kind, TokenKind::OpenBrace);
    assert_eq!(tokens[14].kind, TokenKind::Return);
    assert_eq!(tokens[15].kind, TokenKind::Identifier("a".into()));
    assert_eq!(tokens[16].kind, TokenKind::Plus);
    assert_eq!(tokens[17].kind, TokenKind::Identifier("b".into()));
    assert_eq!(tokens[18].kind, TokenKind::Semicolon);
    assert_eq!(tokens[19].kind, TokenKind::CloseBrace);
    assert_eq!(tokens[20].kind, TokenKind::OptionalChain);
    assert_eq!(tokens[21].kind, TokenKind::NullCoalesce);
    assert_eq!(tokens[22].kind, TokenKind::FatArrow);
    assert_eq!(tokens[23].kind, TokenKind::ColonColon);
    assert_eq!(tokens[24].kind, TokenKind::Eof);
}
