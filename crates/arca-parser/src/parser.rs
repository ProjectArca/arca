//! Pratt parser and recursive descent parser for Arca.

use crate::precedence::Precedence;
use arca_ast::*;
use arca_diagnostics::Diagnostic;
use arca_lexer::{Lexer, Token, TokenKind};

pub struct Parser<'a> {
    lexer: Lexer<'a>,
    current_token: Token,
    peek_token: Token,
    diagnostics: Vec<Diagnostic>,
    file_path: Option<String>,
}

impl<'a> Parser<'a> {
    pub fn new(mut lexer: Lexer<'a>) -> Self {
        let current_token = lexer.next_token();
        let peek_token = lexer.next_token();

        Self {
            lexer,
            current_token,
            peek_token,
            diagnostics: Vec::new(),
            file_path: None,
        }
    }

    pub fn with_file<S: Into<String>>(mut self, file_path: S) -> Self {
        self.file_path = Some(file_path.into());
        self
    }

    pub fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    fn advance(&mut self) {
        self.current_token = std::mem::replace(&mut self.peek_token, self.lexer.next_token());
    }

    fn expect(&mut self, kind: TokenKind) -> bool {
        if self.current_token.kind == kind {
            self.advance();
            true
        } else {
            let diag = Diagnostic::error(format!(
                "Expected token '{}', found '{}'",
                kind, self.current_token.kind
            ))
            .with_span(self.current_token.span);
            self.diagnostics.push(diag);
            false
        }
    }

    pub fn parse_program(&mut self) -> Program {
        let mut declarations = Vec::new();

        while self.current_token.kind != TokenKind::Eof {
            if let Some(decl) = self.parse_declaration() {
                declarations.push(decl);
            } else {
                self.recover();
            }
        }

        Program { declarations }
    }

    fn parse_declaration(&mut self) -> Option<Decl> {
        match &self.current_token.kind {
            TokenKind::Struct => self.parse_struct_decl(),
            TokenKind::Actor => self.parse_struct_decl(),
            TokenKind::Impl => self.parse_impl_decl(),
            TokenKind::Extend => self.parse_extend_decl(),
            TokenKind::Enum | TokenKind::ErrorKw => self.parse_enum_decl(),
            TokenKind::Capability => self.parse_capability_decl(),
            TokenKind::Fn => self.parse_fn_decl().map(Decl::Fn),
            TokenKind::Extern => self.parse_extern_decl(),
            TokenKind::Import => self.parse_import_decl(),
            TokenKind::Let | TokenKind::Const => self.parse_top_var_decl(),
            TokenKind::Export => {
                let start_span = self.current_token.span;
                self.advance(); // export
                let inner = self.parse_declaration()?;
                let end_span = inner_decl_span(&inner);
                Some(Decl::Export {
                    decl: Box::new(inner),
                    span: Span::new(
                        start_span.start,
                        end_span.end,
                        start_span.start_loc,
                        end_span.end_loc,
                    ),
                })
            }
            _ => {
                let diag = Diagnostic::error(format!(
                    "Unexpected top-level token '{}'",
                    self.current_token.kind
                ))
                .with_span(self.current_token.span);
                self.diagnostics.push(diag);
                None
            }
        }
    }

    fn parse_struct_decl(&mut self) -> Option<Decl> {
        let start_span = self.current_token.span;
        self.advance(); // struct

        let name = match &self.current_token.kind {
            TokenKind::Identifier(id) => id.clone(),
            _ => {
                self.diagnostics.push(
                    Diagnostic::error("Expected struct name identifier")
                        .with_span(self.current_token.span),
                );
                return None;
            }
        };
        self.advance();

        if !self.expect(TokenKind::OpenBrace) {
            return None;
        }

        let mut fields = Vec::new();
        let mut methods = Vec::new();

        while self.current_token.kind != TokenKind::CloseBrace
            && self.current_token.kind != TokenKind::Eof
        {
            if self.current_token.kind == TokenKind::Fn {
                if let Some(method) = self.parse_fn_decl() {
                    methods.push(method);
                }
            } else if let TokenKind::Identifier(field_name) = &self.current_token.kind {
                let fname = field_name.clone();
                let fspan = self.current_token.span;
                self.advance();

                // Skip block-like members (e.g. `receive { ... }` in actors)
                if self.current_token.kind == TokenKind::OpenBrace {
                    let mut depth = 1;
                    while depth > 0 && self.current_token.kind != TokenKind::Eof {
                        self.advance();
                        if self.current_token.kind == TokenKind::OpenBrace {
                            depth += 1;
                        } else if self.current_token.kind == TokenKind::CloseBrace {
                            depth -= 1;
                        }
                    }
                    self.advance();
                    continue;
                }

                if !self.expect(TokenKind::Colon) {
                    break;
                }

                let type_ann = match self.parse_type_annotation() {
                    Some(t) => t,
                    None => break,
                };

                fields.push(FieldDef {
                    name: fname,
                    type_ann,
                    span: fspan,
                });

                if self.current_token.kind == TokenKind::Comma {
                    self.advance();
                }
            } else {
                self.advance();
            }
        }

        let end_span = self.current_token.span;
        self.expect(TokenKind::CloseBrace);

        Some(Decl::Struct {
            name,
            fields,
            methods,
            span: Span::new(
                start_span.start,
                end_span.end,
                start_span.start_loc,
                end_span.end_loc,
            ),
        })
    }

    fn parse_impl_decl(&mut self) -> Option<Decl> {
        let start_span = self.current_token.span;
        self.advance(); // impl

        let target_name = match &self.current_token.kind {
            TokenKind::Identifier(id) => id.clone(),
            _ => {
                self.diagnostics.push(
                    Diagnostic::error("Expected type name after 'impl'")
                        .with_span(self.current_token.span),
                );
                return None;
            }
        };
        self.advance();

        if !self.expect(TokenKind::OpenBrace) {
            return None;
        }

        let mut methods = Vec::new();
        while self.current_token.kind != TokenKind::CloseBrace
            && self.current_token.kind != TokenKind::Eof
        {
            if self.current_token.kind == TokenKind::Fn {
                if let Some(method) = self.parse_fn_decl() {
                    methods.push(method);
                }
            } else {
                self.advance();
            }
        }

        let end_span = self.current_token.span;
        self.expect(TokenKind::CloseBrace);

        Some(Decl::Extend {
            target_name,
            methods,
            span: Span::new(
                start_span.start,
                end_span.end,
                start_span.start_loc,
                end_span.end_loc,
            ),
        })
    }

    fn parse_extend_decl(&mut self) -> Option<Decl> {
        let start_span = self.current_token.span;
        self.advance(); // extend

        let target_name = match &self.current_token.kind {
            TokenKind::Identifier(id) => id.clone(),
            _ => {
                self.diagnostics.push(
                    Diagnostic::error("Expected target type identifier after 'extend'")
                        .with_span(self.current_token.span),
                );
                return None;
            }
        };
        self.advance();

        // Skip optional generic args like Array<User>
        if self.current_token.kind == TokenKind::Less {
            let mut depth = 1;
            while depth > 0 && self.current_token.kind != TokenKind::Eof {
                self.advance();
                if self.current_token.kind == TokenKind::Less {
                    depth += 1;
                } else if self.current_token.kind == TokenKind::Greater {
                    depth -= 1;
                }
            }
            self.advance();
        }

        if !self.expect(TokenKind::OpenBrace) {
            return None;
        }

        let mut methods = Vec::new();

        while self.current_token.kind != TokenKind::CloseBrace
            && self.current_token.kind != TokenKind::Eof
        {
            if self.current_token.kind == TokenKind::Fn {
                if let Some(method) = self.parse_fn_decl() {
                    methods.push(method);
                }
            } else {
                self.advance();
            }
        }

        let end_span = self.current_token.span;
        self.expect(TokenKind::CloseBrace);

        Some(Decl::Extend {
            target_name,
            methods,
            span: Span::new(
                start_span.start,
                end_span.end,
                start_span.start_loc,
                end_span.end_loc,
            ),
        })
    }

    fn parse_enum_decl(&mut self) -> Option<Decl> {
        let start_span = self.current_token.span;
        self.advance(); // enum

        let name = match &self.current_token.kind {
            TokenKind::Identifier(id) => id.clone(),
            _ => return None,
        };
        self.advance();

        if !self.expect(TokenKind::OpenBrace) {
            return None;
        }

        let mut variants = Vec::new();

        while self.current_token.kind != TokenKind::CloseBrace
            && self.current_token.kind != TokenKind::Eof
        {
            if let TokenKind::Identifier(vname) = &self.current_token.kind {
                let vn = vname.clone();
                let vspan = self.current_token.span;
                self.advance();

                let mut payload = Vec::new();
                if self.current_token.kind == TokenKind::OpenParen {
                    self.advance();
                    while self.current_token.kind != TokenKind::CloseParen
                        && self.current_token.kind != TokenKind::Eof
                    {
                        // Only skip named params `name: type`, not bare types
                        if let TokenKind::Identifier(_) = &self.current_token.kind {
                            if self.peek_token.kind == TokenKind::Colon {
                                self.advance(); // skip param name
                                self.advance(); // skip :
                            }
                        }
                        if let Some(t) = self.parse_type_annotation() {
                            payload.push(t);
                        } else {
                            self.advance();
                        }
                        if self.current_token.kind == TokenKind::Comma {
                            self.advance();
                        }
                    }
                    self.expect(TokenKind::CloseParen);
                }

                variants.push(EnumVariantDef {
                    name: vn,
                    payload,
                    span: vspan,
                });

                if self.current_token.kind == TokenKind::Comma {
                    self.advance();
                }
            } else {
                self.advance();
            }
        }

        let end_span = self.current_token.span;
        self.expect(TokenKind::CloseBrace);

        Some(Decl::Enum {
            name,
            variants,
            span: Span::new(
                start_span.start,
                end_span.end,
                start_span.start_loc,
                end_span.end_loc,
            ),
        })
    }

    fn parse_capability_decl(&mut self) -> Option<Decl> {
        let start_span = self.current_token.span;
        self.advance(); // capability

        let name = match &self.current_token.kind {
            TokenKind::Identifier(id) => id.clone(),
            _ => return None,
        };
        self.advance();

        if !self.expect(TokenKind::OpenBrace) {
            return None;
        }

        let mut methods = Vec::new();

        while self.current_token.kind != TokenKind::CloseBrace
            && self.current_token.kind != TokenKind::Eof
        {
            if self.current_token.kind == TokenKind::Fn {
                let mstart = self.current_token.span;
                self.advance(); // fn

                let mname = match &self.current_token.kind {
                    TokenKind::Identifier(id) => id.clone(),
                    _ => break,
                };
                self.advance();

                let params = self.parse_fn_params();
                let return_type = if self.current_token.kind == TokenKind::Arrow {
                    self.advance();
                    self.parse_type_annotation()
                } else {
                    None
                };

                let mend = self.current_token.span;
                if self.current_token.kind == TokenKind::Semicolon {
                    self.advance();
                }

                methods.push(CapabilityMethodDef {
                    name: mname,
                    params,
                    return_type,
                    span: Span::new(
                        mstart.start,
                        mend.end,
                        mstart.start_loc,
                        mend.end_loc,
                    ),
                });
            } else {
                self.advance();
            }
        }

        let end_span = self.current_token.span;
        self.expect(TokenKind::CloseBrace);

        Some(Decl::Capability {
            name,
            methods,
            span: Span::new(
                start_span.start,
                end_span.end,
                start_span.start_loc,
                end_span.end_loc,
            ),
        })
    }

    fn parse_fn_decl(&mut self) -> Option<FnDecl> {
        let start_span = self.current_token.span;
        self.advance(); // fn

        let name = match &self.current_token.kind {
            TokenKind::Identifier(id) => id.clone(),
            _ => {
                self.diagnostics.push(
                    Diagnostic::error("Expected function name identifier")
                        .with_span(self.current_token.span),
                );
                return None;
            }
        };
        self.advance();

        let params = self.parse_fn_params();

        let return_type = if self.current_token.kind == TokenKind::Arrow {
            self.advance();
            self.parse_type_annotation()
        } else {
            None
        };

        let throws_type = if let TokenKind::Identifier(s) = &self.current_token.kind {
            if s == "throws" {
                self.advance();
                self.parse_type_annotation()
            } else {
                None
            }
        } else {
            None
        };

        let body = self.parse_block_expr()?;
        let end_span = body.span;

        Some(FnDecl {
            name,
            params,
            return_type,
            throws_type,
            body,
            span: Span::new(
                start_span.start,
                end_span.end,
                start_span.start_loc,
                end_span.end_loc,
            ),
        })
    }

    fn parse_extern_decl(&mut self) -> Option<Decl> {
        let start_span = self.current_token.span;
        self.advance(); // extern

        if !self.expect(TokenKind::Fn) {
            return None;
        }

        let name = match &self.current_token.kind {
            TokenKind::Identifier(id) => id.clone(),
            _ => {
                self.diagnostics.push(
                    Diagnostic::error("Expected extern function name identifier")
                        .with_span(self.current_token.span),
                );
                return None;
            }
        };
        self.advance();

        let params = self.parse_fn_params();

        let return_type = if self.current_token.kind == TokenKind::Arrow {
            self.advance();
            self.parse_type_annotation()
        } else {
            None
        };

        let body = if self.current_token.kind == TokenKind::OpenBrace {
            self.advance();
            let body = match &self.current_token.kind {
                TokenKind::StringLiteral(s) => s.clone(),
                _ => {
                    self.diagnostics.push(
                        Diagnostic::error("Expected string literal for extern function body")
                            .with_span(self.current_token.span),
                    );
                    return None;
                }
            };
            self.advance();
            self.expect(TokenKind::CloseBrace);
            body
        } else {
            String::new()
        };

        let end_span = self.current_token.span;
        Some(Decl::Extern {
            name,
            params,
            return_type,
            body,
            span: Span::new(
                start_span.start,
                end_span.end,
                start_span.start_loc,
                end_span.end_loc,
            ),
        })
    }

    fn parse_fn_params(&mut self) -> Vec<ParamDef> {
        let mut params = Vec::new();
        if !self.expect(TokenKind::OpenParen) {
            return params;
        }

        while self.current_token.kind != TokenKind::CloseParen
            && self.current_token.kind != TokenKind::Eof
        {
            if let TokenKind::Identifier(pname) = &self.current_token.kind {
                let pn = pname.clone();
                let pspan = self.current_token.span;
                self.advance();

                if self.current_token.kind == TokenKind::Colon {
                    self.advance();
                    if let Some(t) = self.parse_type_annotation() {
                        params.push(ParamDef {
                            name: pn,
                            type_ann: t,
                            span: pspan,
                        });
                    }
                } else if pn == "self" {
                    params.push(ParamDef {
                        name: pn,
                        type_ann: TypeAnnotation::Named("Self".into()),
                        span: pspan,
                    });
                }
                if self.current_token.kind == TokenKind::Comma {
                    self.advance();
                }
            } else {
                self.advance();
            }
        }

        self.expect(TokenKind::CloseParen);
        params
    }

    fn parse_import_decl(&mut self) -> Option<Decl> {
        let start_span = self.current_token.span;
        self.advance(); // import

        let mut namespace = None;
        let mut items = Vec::new();
        if self.current_token.kind == TokenKind::OpenBrace {
            self.advance();
            while self.current_token.kind != TokenKind::CloseBrace
                && self.current_token.kind != TokenKind::Eof
            {
                if let TokenKind::Identifier(item) = &self.current_token.kind {
                    items.push(item.clone());
                    self.advance();
                }
                if self.current_token.kind == TokenKind::Comma {
                    self.advance();
                }
            }
            self.expect(TokenKind::CloseBrace);
        } else if let TokenKind::Identifier(name) = &self.current_token.kind {
            if name != "from" {
                namespace = Some(name.clone());
                self.advance();
            }
        }

        let _from = self.expect(TokenKind::Identifier("from".into()));

        let source = match &self.current_token.kind {
            TokenKind::StringLiteral(s) => s.clone(),
            _ => "".into(),
        };
        let end_span = self.current_token.span;
        self.advance();

        Some(Decl::Import {
            namespace,
            items,
            source,
            span: Span::new(
                start_span.start,
                end_span.end,
                start_span.start_loc,
                end_span.end_loc,
            ),
        })
    }

    fn parse_top_var_decl(&mut self) -> Option<Decl> {
        let is_const = self.current_token.kind == TokenKind::Const;
        let start_span = self.current_token.span;
        self.advance(); // let / const

        let name = match &self.current_token.kind {
            TokenKind::Identifier(id) => id.clone(),
            _ => return None,
        };
        let name_span = self.current_token.span;
        self.advance();

        let type_ann = if self.current_token.kind == TokenKind::Colon {
            self.advance();
            self.parse_type_annotation()
        } else {
            None
        };

        let init = if self.current_token.kind == TokenKind::Assign {
            self.advance();
            self.parse_expression(Precedence::Lowest)
        } else {
            None
        };

        let end_span = self.current_token.span;

        if is_const {
            if let Some(init_val) = init {
                let ret_type = type_ann.unwrap_or(TypeAnnotation::Named("void".into()));
                Some(Decl::Fn(FnDecl {
                    name,
                    params: Vec::new(),
                    return_type: Some(ret_type),
                    throws_type: None,
                    body: BlockExpr {
                        statements: Vec::new(),
                        final_expr: Some(Box::new(init_val)),
                        span: Span::new(start_span.start, end_span.end, start_span.start_loc, end_span.end_loc),
                    },
                    span: Span::new(start_span.start, end_span.end, start_span.start_loc, end_span.end_loc),
                }))
            } else {
                None
            }
        } else {
            None
        }
    }

    fn parse_type_annotation(&mut self) -> Option<TypeAnnotation> {
        let first = self.parse_single_type()?;
        if self.current_token.kind == TokenKind::Pipe {
            let mut variants = vec![first];
            while self.current_token.kind == TokenKind::Pipe {
                self.advance();
                if let Some(t) = self.parse_single_type() {
                    variants.push(t);
                }
            }
            Some(TypeAnnotation::Union(variants))
        } else {
            Some(first)
        }
    }

    fn parse_single_type(&mut self) -> Option<TypeAnnotation> {
        match &self.current_token.kind {
            TokenKind::Identifier(name) => {
                let n = name.clone();
                self.advance();

                let base = if self.current_token.kind == TokenKind::Less {
                    self.advance();
                    let mut args = Vec::new();
                    while self.current_token.kind != TokenKind::Greater
                        && self.current_token.kind != TokenKind::Eof
                    {
                        if let Some(t) = self.parse_type_annotation() {
                            args.push(t);
                        }
                        if self.current_token.kind == TokenKind::Comma {
                            self.advance();
                        }
                    }
                    self.expect(TokenKind::Greater);
                    // ref<T> → Ref, ptr<T> → Ptr
                    if n == "ref" && args.len() == 1 {
                        Some(TypeAnnotation::Ref { inner: Box::new(args.into_iter().next().unwrap()) })
                    } else if n == "ptr" && args.len() == 1 {
                        Some(TypeAnnotation::Ptr { inner: Box::new(args.into_iter().next().unwrap()) })
                    } else {
                        Some(TypeAnnotation::Generic { name: n, args })
                    }
                } else {
                    Some(TypeAnnotation::Named(n))
                };

                // Handle T? → Option<T>
                if self.current_token.kind == TokenKind::Question {
                    self.advance();
                    let inner = base?;
                    Some(TypeAnnotation::Generic {
                        name: "Option".into(),
                        args: vec![inner],
                    })
                } else {
                    base
                }
            }
            _ => None,
        }
    }

    // --- Pratt Parser Engine ---

    pub fn parse_expression(&mut self, precedence: Precedence) -> Option<Expr> {
        let mut left = self.parse_prefix_expression()?;

        while self.current_token.kind != TokenKind::Semicolon
            && precedence < Precedence::for_token(&self.current_token.kind)
        {
            left = match self.parse_infix_expression(left.clone()) {
                Some(expr) => expr,
                None => return Some(left),
            };
        }

        Some(left)
    }

    fn parse_prefix_expression(&mut self) -> Option<Expr> {
        let token = self.current_token.clone();
        match &token.kind {
            TokenKind::IntLiteral(n) => {
                self.advance();
                Some(Expr::Literal {
                    value: LiteralKind::Int(*n),
                    span: token.span,
                })
            }
            TokenKind::FloatLiteral(f) => {
                self.advance();
                Some(Expr::Literal {
                    value: LiteralKind::Float(*f),
                    span: token.span,
                })
            }
            TokenKind::StringLiteral(s) => {
                self.advance();
                Some(Expr::Literal {
                    value: LiteralKind::String(s.clone()),
                    span: token.span,
                })
            }
            TokenKind::CharLiteral(c) => {
                self.advance();
                Some(Expr::Literal {
                    value: LiteralKind::Char(*c),
                    span: token.span,
                })
            }
            TokenKind::True => {
                self.advance();
                Some(Expr::Literal {
                    value: LiteralKind::Bool(true),
                    span: token.span,
                })
            }
            TokenKind::False => {
                self.advance();
                Some(Expr::Literal {
                    value: LiteralKind::Bool(false),
                    span: token.span,
                })
            }
            TokenKind::NoneKw => {
                self.advance();
                Some(Expr::Literal {
                    value: LiteralKind::None,
                    span: token.span,
                })
            }
            TokenKind::Identifier(id) => {
                let name = id.clone();
                self.advance();

                // Check for intrinsic prefix (e.g. `@borrow` or `@move`)
                if name.starts_with('@') {
                    if self.current_token.kind == TokenKind::OpenParen {
                        self.advance();
                        let mut args = Vec::new();
                        while self.current_token.kind != TokenKind::CloseParen
                            && self.current_token.kind != TokenKind::Eof
                        {
                            if let Some(arg) = self.parse_expression(Precedence::Lowest) {
                                args.push(arg);
                            } else {
                                self.advance();
                            }
                            if self.current_token.kind == TokenKind::Comma {
                                self.advance();
                            }
                        }
                        let end_span = self.current_token.span;
                        self.expect(TokenKind::CloseParen);
                        return Some(Expr::IntrinsicCall {
                            name,
                            args,
                            span: Span::new(
                                token.span.start,
                                end_span.end,
                                token.span.start_loc,
                                end_span.end_loc,
                            ),
                        });
                    }
                }

                // Check for struct literal: `User { name, age }` or `User { name: value }`
                if self.current_token.kind == TokenKind::OpenBrace
                    && name.chars().next().map_or(false, |c| c.is_uppercase())
                {
                    return self.parse_struct_literal(name, token.span);
                }

                // Skip generic type args in expressions: `Channel<i32>()`
                if self.current_token.kind == TokenKind::Less
                    && name.chars().next().map_or(false, |c| c.is_uppercase())
                {
                    let mut depth = 1;
                    while depth > 0 && self.current_token.kind != TokenKind::Eof {
                        self.advance();
                        if self.current_token.kind == TokenKind::Less {
                            depth += 1;
                        } else if self.current_token.kind == TokenKind::Greater {
                            depth -= 1;
                        }
                    }
                    self.advance();
                }

                Some(Expr::Identifier {
                    name,
                    span: token.span,
                })
            }
            TokenKind::OpenParen => {
                // Save state to potentially backtrack
                let saved_lex = self.lexer.save();
                let saved_current = self.current_token.clone();
                let saved_peek = self.peek_token.clone();

                self.advance();
                // Try zero-param closure: () => expr
                if self.current_token.kind == TokenKind::CloseParen
                    && self.peek_token.kind == TokenKind::FatArrow
                {
                    self.advance(); // )
                    self.advance(); // =>
                    let body = self.parse_expression(Precedence::Lowest)?;
                    let end_span = body.span();
                    return Some(Expr::Closure {
                        params: Vec::new(),
                        body: Box::new(body),
                        span: Span::new(
                            token.span.start, end_span.end,
                            token.span.start_loc, end_span.end_loc,
                        ),
                    });
                }
                // Try param closure: (ident, ...) => or (ident: Type, ...) =>
                let mut params = Vec::new();
                let mut ok = true;
                loop {
                    match &self.current_token.kind {
                        TokenKind::CloseParen => {
                            self.advance();
                            break;
                        }
                        TokenKind::Identifier(pname) => {
                            let pn = pname.clone();
                            let pspan = self.current_token.span;
                            self.advance();
                            let type_ann = if self.current_token.kind == TokenKind::Colon {
                                self.advance();
                                self.parse_type_annotation()
                            } else {
                                None
                            };
                            params.push(ParamDef {
                                name: pn,
                                type_ann: type_ann.unwrap_or(TypeAnnotation::Named("i64".into())),
                                span: pspan,
                            });
                            if self.current_token.kind == TokenKind::Comma {
                                self.advance();
                            }
                        }
                        _ => { ok = false; break; }
                    }
                }
                if ok && self.current_token.kind == TokenKind::FatArrow {
                    self.advance(); // =>
                    let body = self.parse_expression(Precedence::Lowest)?;
                    let end_span = body.span();
                    return Some(Expr::Closure {
                        params,
                        body: Box::new(body),
                        span: Span::new(
                            token.span.start, end_span.end,
                            token.span.start_loc, end_span.end_loc,
                        ),
                    });
                }
                // Not a closure: restore lexer and parse as paren expr
                self.lexer.restore(saved_lex);
                self.current_token = saved_current;
                self.peek_token = saved_peek;
                self.advance(); // consume (
                let expr = self.parse_expression(Precedence::Lowest)?;
                self.expect(TokenKind::CloseParen);
                Some(expr)
            }
            TokenKind::Minus => {
                self.advance();
                let right = self.parse_expression(Precedence::Unary)?;
                let end_span = right.span();
                Some(Expr::Unary {
                    op: UnaryOp::Neg,
                    span: Span::new(
                        token.span.start,
                        end_span.end,
                        token.span.start_loc,
                        end_span.end_loc,
                    ),
                    expr: Box::new(right),
                })
            }
            TokenKind::Not => {
                self.advance();
                let right = self.parse_expression(Precedence::Unary)?;
                let end_span = right.span();
                Some(Expr::Unary {
                    op: UnaryOp::Not,
                    span: Span::new(
                        token.span.start,
                        end_span.end,
                        token.span.start_loc,
                        end_span.end_loc,
                    ),
                    expr: Box::new(right),
                })
            }
            TokenKind::If => self.parse_if_expr(),
            TokenKind::Match => self.parse_match_expr(),
            TokenKind::Comptime => {
                self.advance(); // comptime
                if self.current_token.kind == TokenKind::OpenBrace {
                    let block = self.parse_block_expr()?;
                    Some(Expr::ComptimeBlock {
                        span: Span::new(
                            token.span.start,
                            block.span.end,
                            token.span.start_loc,
                            block.span.end_loc,
                        ),
                        body: block,
                    })
                } else if let Some(expr) = self.parse_expression(Precedence::Primary) {
                    let end_span = expr.span();
                    Some(Expr::ComptimeBlock {
                        span: Span::new(
                            token.span.start,
                            end_span.end,
                            token.span.start_loc,
                            end_span.end_loc,
                        ),
                        body: BlockExpr {
                            statements: Vec::new(),
                            final_expr: Some(Box::new(expr)),
                            span: end_span,
                        },
                    })
                } else {
                    None
                }
            }
            TokenKind::Loop => {
                self.advance(); // loop
                let block = self.parse_block_expr()?;
                Some(Expr::Loop {
                    span: Span::new(
                        token.span.start,
                        block.span.end,
                        token.span.start_loc,
                        block.span.end_loc,
                    ),
                    body: block,
                })
            }
            TokenKind::Move | TokenKind::Borrow => {
                let is_borrow = token.kind == TokenKind::Borrow;
                let name = if is_borrow { "borrow".to_string() } else { "move".to_string() };
                self.advance(); // move or borrow
                if self.current_token.kind == TokenKind::OpenParen {
                    self.advance(); // (
                    let inner = self.parse_expression(Precedence::Lowest)?;
                    let end_span = self.current_token.span;
                    self.expect(TokenKind::CloseParen);
                    Some(Expr::Call {
                        callee: Box::new(Expr::Identifier { name, span: token.span }),
                        args: vec![inner],
                        span: Span::new(token.span.start, end_span.end, token.span.start_loc, end_span.end_loc),
                    })
                } else if let Some(inner) = self.parse_expression(Precedence::Primary) {
                    let end_span = inner.span();
                    Some(Expr::Call {
                        callee: Box::new(Expr::Identifier { name, span: token.span }),
                        args: vec![inner],
                        span: Span::new(token.span.start, end_span.end, token.span.start_loc, end_span.end_loc),
                    })
                } else {
                    Some(Expr::Identifier { name, span: token.span })
                }
            }
            TokenKind::Spawn => {
                self.advance(); // spawn
                if self.current_token.kind == TokenKind::OpenBrace {
                    let block = self.parse_block_expr()?;
                    Some(Expr::SpawnBlock {
                        span: Span::new(
                            token.span.start,
                            block.span.end,
                            token.span.start_loc,
                            block.span.end_loc,
                        ),
                        body: block,
                    })
                } else if let Some(expr) = self.parse_expression(Precedence::Primary) {
                    let end_span = expr.span();
                    Some(Expr::SpawnBlock {
                        span: Span::new(
                            token.span.start,
                            end_span.end,
                            token.span.start_loc,
                            end_span.end_loc,
                        ),
                        body: BlockExpr {
                            statements: Vec::new(),
                            final_expr: Some(Box::new(expr)),
                            span: end_span,
                        },
                    })
                } else {
                    None
                }
            }
            TokenKind::Try => {
                self.advance(); // try
                let block = self.parse_block_expr()?;
                // Optional catch block: catch err { ... } or catch { ... }
                let catch_var = if self.current_token.kind == TokenKind::Catch {
                    self.advance(); // catch
                    match &self.current_token.kind {
                        TokenKind::Identifier(name) => {
                            let n = name.clone();
                            self.advance();
                            Some(n)
                        }
            _ => None,
                    }
                } else {
                    None
                };
                if self.current_token.kind == TokenKind::OpenBrace {
                    let catch_block = self.parse_block_expr()?;
                    let catch_span = catch_block.span;
                    let block_span = block.span;
                    let try_expr = Expr::Block(block);
                    let catch_expr = if let Some(var) = catch_var {
                        Expr::Block(BlockExpr {
                            statements: vec![Stmt::VarDecl {
                                is_const: true,
                                name: var,
                                type_ann: None,
                                init: Some(Expr::Identifier {
                                    name: "__catch_err".into(),
                                    span: catch_span,
                                }),
                                span: catch_span,
                            }],
                            final_expr: catch_block.final_expr,
                            span: catch_span,
                        })
                    } else {
                        Expr::Block(catch_block)
                    };
                    Some(Expr::GroupBlock {
                        body: BlockExpr {
                            statements: vec![
                                Stmt::Expr { expr: try_expr, has_semicolon: false, span: block_span },
                            ],
                            final_expr: Some(Box::new(catch_expr)),
                            span: Span::new(
                                token.span.start,
                                catch_span.end,
                                token.span.start_loc,
                                catch_span.end_loc,
                            ),
                        },
                        span: Span::new(
                            token.span.start,
                            catch_span.end,
                            token.span.start_loc,
                            catch_span.end_loc,
                        ),
                    })
                } else {
                    Some(Expr::TryBlock {
                        span: Span::new(
                            token.span.start,
                            block.span.end,
                            token.span.start_loc,
                            block.span.end_loc,
                        ),
                        body: block,
                    })
                }
            }
            TokenKind::Group => {
                self.advance(); // group
                let block = self.parse_block_expr()?;
                Some(Expr::GroupBlock {
                    span: Span::new(
                        token.span.start,
                        block.span.end,
                        token.span.start_loc,
                        block.span.end_loc,
                    ),
                    body: block,
                })
            }
            TokenKind::ThrowKw => {
                let start = self.current_token.span;
                self.advance();
                let value = self.parse_expression(Precedence::Lowest)?;
                let end = value.span();
                Some(Expr::Throw {
                    value: Box::new(value),
                    span: Span::new(start.start, end.end, start.start_loc, end.end_loc),
                })
            }
            TokenKind::OpenBrace => {
                self.parse_block_expr().map(Expr::Block)
            }
            _ => None,
        }
    }

    fn parse_struct_literal(&mut self, struct_name: String, start_span: Span) -> Option<Expr> {
        self.advance(); // {
        let mut fields = Vec::new();

        while self.current_token.kind != TokenKind::CloseBrace
            && self.current_token.kind != TokenKind::Eof
        {
            if let TokenKind::Identifier(fname) = &self.current_token.kind {
                let name = fname.clone();
                let fspan = self.current_token.span;
                self.advance();

                // Method shorthand: `fetch(req) { body }` → field with closure value
                if self.current_token.kind == TokenKind::OpenParen {
                    self.advance(); // (
                    let mut params = Vec::new();
                    while self.current_token.kind != TokenKind::CloseParen
                        && self.current_token.kind != TokenKind::Eof
                    {
                        if let TokenKind::Identifier(pname) = &self.current_token.kind {
                            let pname = pname.clone();
                            let pspan = self.current_token.span;
                            self.advance();
                            let type_ann = if self.current_token.kind == TokenKind::Colon {
                                self.advance();
                                self.parse_type_annotation()
                            } else {
                                None
                            };
                            params.push(ParamDef {
                                name: pname.clone(),
                                type_ann: type_ann.unwrap_or(TypeAnnotation::Named("string".into())),
                                span: pspan,
                            });
                            if self.current_token.kind == TokenKind::Comma {
                                self.advance();
                            }
                        } else {
                            self.advance();
                        }
                    }
                    self.expect(TokenKind::CloseParen);
                    let body = self.parse_block_expr()?;
                    let body_block = Expr::Block(body);
                    fields.push(StructFieldInit {
                        name,
                        value: Some(Expr::Closure {
                            params,
                            body: Box::new(body_block),
                            span: Span::new(fspan.start, fspan.end, fspan.start_loc, fspan.end_loc),
                        }),
                        span: fspan,
                    });
                } else {
                    let value = if self.current_token.kind == TokenKind::Colon {
                        self.advance();
                        self.parse_expression(Precedence::Lowest)
                    } else {
                        // Property shorthand: `User { name }` -> `name: name`
                        None
                    };

                    fields.push(StructFieldInit {
                        name,
                        value,
                        span: fspan,
                    });

                    if self.current_token.kind == TokenKind::Comma {
                        self.advance();
                    }
                }
            } else {
                self.advance();
            }
        }

        let end_span = self.current_token.span;
        self.expect(TokenKind::CloseBrace);

        Some(Expr::StructLiteral {
            name: struct_name,
            fields,
            span: Span::new(
                start_span.start,
                end_span.end,
                start_span.start_loc,
                end_span.end_loc,
            ),
        })
    }

    /// Parse `{ ident: expr, ident(args) { body }, ... }` as anonymous struct literal
    fn parse_anonymous_struct(&mut self, start_span: Span, already_consumed: bool) -> Option<Expr> {
        if !already_consumed {
            self.advance(); // {
        }
        let mut fields = Vec::new();
        while self.current_token.kind != TokenKind::CloseBrace
            && self.current_token.kind != TokenKind::Eof
        {
            if let TokenKind::Identifier(fname) = &self.current_token.kind {
                let name = fname.clone();
                let fspan = self.current_token.span;
                self.advance();

                if self.current_token.kind == TokenKind::OpenParen {
                    // Method shorthand: fetch(req) { ... }
                    self.advance();
                    let mut params = Vec::new();
                    while self.current_token.kind != TokenKind::CloseParen
                        && self.current_token.kind != TokenKind::Eof
                    {
                        if let TokenKind::Identifier(pname) = &self.current_token.kind {
                            let pname = pname.clone();
                            let pspan = self.current_token.span;
                            self.advance();
                            params.push(ParamDef {
                                name: pname,
                                type_ann: TypeAnnotation::Named("string".into()),
                                span: pspan,
                            });
                            if self.current_token.kind == TokenKind::Comma {
                                self.advance();
                            }
                        } else { self.advance(); }
                    }
                    self.expect(TokenKind::CloseParen);
                    let body = self.parse_block_expr()?;
                    fields.push(StructFieldInit {
                        name,
                        value: Some(Expr::Closure {
                            params,
                            body: Box::new(Expr::Block(body)),
                            span: Span::new(fspan.start, fspan.end, fspan.start_loc, fspan.end_loc),
                        }),
                        span: fspan,
                    });
                } else if self.current_token.kind == TokenKind::Colon {
                    self.advance();
                    if let Some(val) = self.parse_expression(Precedence::Lowest) {
                        fields.push(StructFieldInit { name, value: Some(val), span: fspan });
                    }
                } else {
                    // Shorthand
                    fields.push(StructFieldInit { name, value: None, span: fspan });
                }

                if self.current_token.kind == TokenKind::Comma {
                    self.advance();
                }
            } else {
                self.advance();
            }
        }
        let end_span = self.current_token.span;
        self.expect(TokenKind::CloseBrace);

        Some(Expr::StructLiteral {
            name: String::new(), // anonymous
            fields,
            span: Span::new(start_span.start, end_span.end, start_span.start_loc, end_span.end_loc),
        })
    }

    /// Parse `{ ... }` as anonymous struct (called from parse_block_expr fast path)
    /// Already past `{`, current_token is the first content token.
    fn parse_anonymous_struct_here(&mut self, start_span: Span) -> Option<BlockExpr> {
        let anon_span = Span::new(start_span.start, start_span.end, start_span.start_loc, start_span.end_loc);
        self.parse_anonymous_struct(anon_span, true).map(|struct_expr| BlockExpr {
            statements: vec![Stmt::Expr {
                expr: struct_expr,
                has_semicolon: false,
                span: anon_span,
            }],
            final_expr: None,
            span: anon_span,
        })
    }

    fn parse_if_expr(&mut self) -> Option<Expr> {
        let start_span = self.current_token.span;
        self.advance(); // if

        // Check for `if let pattern = expr` syntax
        if self.current_token.kind == TokenKind::Let {
            self.advance(); // let
            let pattern = self.parse_pattern()?;
            self.expect(TokenKind::Assign);
            let value = self.parse_expression(Precedence::Lowest)?;
            let then_branch = self.parse_block_expr()?;

            let else_branch = if self.current_token.kind == TokenKind::Else {
                self.advance();
                if self.current_token.kind == TokenKind::If {
                    self.parse_if_expr()
                } else {
                    self.parse_block_expr().map(Expr::Block)
                }
            } else {
                None
            };

            let end_span = else_branch
                .as_ref()
                .map(|e| e.span())
                .unwrap_or(then_branch.span);

            // Desugar if-let into match expression
            return Some(Expr::Match {
                value: Box::new(value),
                arms: vec![
                    MatchArm {
                        pattern,
                        guard: None,
                        body: Expr::Block(then_branch),
                        span: Span::new(
                            start_span.start,
                            end_span.end,
                            start_span.start_loc,
                            end_span.end_loc,
                        ),
                    },
                    MatchArm {
                        pattern: Pattern::Wildcard,
                        guard: None,
                        body: else_branch.unwrap_or(Expr::Block(BlockExpr {
                            statements: Vec::new(),
                            final_expr: None,
                            span: end_span,
                        })),
                        span: Span::new(
                            start_span.start,
                            end_span.end,
                            start_span.start_loc,
                            end_span.end_loc,
                        ),
                    },
                ],
                span: Span::new(
                    start_span.start,
                    end_span.end,
                    start_span.start_loc,
                    end_span.end_loc,
                ),
            });
        }

        let cond = self.parse_expression(Precedence::Lowest)?;
        let then_branch = self.parse_block_expr()?;

        let else_branch = if self.current_token.kind == TokenKind::Else {
            self.advance();
            if self.current_token.kind == TokenKind::If {
                self.parse_if_expr()
            } else {
                self.parse_block_expr().map(Expr::Block)
            }
        } else {
            None
        };

        let end_span = else_branch
            .as_ref()
            .map(|e| e.span())
            .unwrap_or(then_branch.span);

        Some(Expr::If {
            cond: Box::new(cond),
            then_branch,
            else_branch: else_branch.map(Box::new),
            span: Span::new(
                start_span.start,
                end_span.end,
                start_span.start_loc,
                end_span.end_loc,
            ),
        })
    }

    fn parse_match_expr(&mut self) -> Option<Expr> {
        let start_span = self.current_token.span;
        self.advance(); // match

        let value = self.parse_expression(Precedence::Lowest)?;
        if !self.expect(TokenKind::OpenBrace) {
            return None;
        }

        let mut arms = Vec::new();
        while self.current_token.kind != TokenKind::CloseBrace
            && self.current_token.kind != TokenKind::Eof
        {
            let arm_start = self.current_token.span;
            let pattern = self.parse_pattern()?;

            let guard = if self.current_token.kind == TokenKind::If {
                self.advance();
                self.parse_expression(Precedence::Lowest)
            } else {
                None
            };

            self.expect(TokenKind::FatArrow);
            let body = self.parse_expression(Precedence::Lowest)?;
            let arm_end = body.span();

            arms.push(MatchArm {
                pattern,
                guard,
                body,
                span: Span::new(
                    arm_start.start,
                    arm_end.end,
                    arm_start.start_loc,
                    arm_end.end_loc,
                ),
            });

            if self.current_token.kind == TokenKind::Comma {
                self.advance();
            }
        }

        let end_span = self.current_token.span;
        self.expect(TokenKind::CloseBrace);

        Some(Expr::Match {
            value: Box::new(value),
            arms,
            span: Span::new(
                start_span.start,
                end_span.end,
                start_span.start_loc,
                end_span.end_loc,
            ),
        })
    }

    fn parse_pattern(&mut self) -> Option<Pattern> {
        match &self.current_token.kind {
            TokenKind::IntLiteral(n) => {
                let val = *n;
                self.advance();
                Some(Pattern::Literal(LiteralKind::Int(val)))
            }
            TokenKind::StringLiteral(s) => {
                let val = s.clone();
                self.advance();
                Some(Pattern::Literal(LiteralKind::String(val)))
            }
            TokenKind::Identifier(id) => {
                let name = id.clone();
                self.advance();
                if self.current_token.kind == TokenKind::Dot {
                    self.advance(); // .
                    let variant = match &self.current_token.kind {
                        TokenKind::Identifier(v) => v.clone(),
                        _ => return None,
                    };
                    self.advance();
                    let mut inner = Vec::new();
                    if self.current_token.kind == TokenKind::OpenParen {
                        self.advance();
                        while self.current_token.kind != TokenKind::CloseParen
                            && self.current_token.kind != TokenKind::Eof
                        {
                            if let Some(p) = self.parse_pattern() {
                                inner.push(p);
                            } else {
                                self.advance();
                            }
                            if self.current_token.kind == TokenKind::Comma {
                                self.advance();
                            }
                        }
                        self.expect(TokenKind::CloseParen);
                    }
                    Some(Pattern::Variant {
                        enum_name: Some(name),
                        variant,
                        inner,
                    })
                } else if name == "_" {
                    Some(Pattern::Wildcard)
                } else if self.current_token.kind == TokenKind::OpenParen {
                    self.advance();
                    let mut inner = Vec::new();
                    while self.current_token.kind != TokenKind::CloseParen
                        && self.current_token.kind != TokenKind::Eof
                    {
                        if let Some(p) = self.parse_pattern() {
                            inner.push(p);
                        } else {
                            self.advance();
                        }
                        if self.current_token.kind == TokenKind::Comma {
                            self.advance();
                        }
                    }
                    self.expect(TokenKind::CloseParen);
                    Some(Pattern::Variant {
                        enum_name: None,
                        variant: name,
                        inner,
                    })
                } else {
                    Some(Pattern::Identifier(name))
                }
            }
            _ => None,
        }
    }

    fn parse_infix_expression(&mut self, left: Expr) -> Option<Expr> {
        let op_token = self.current_token.clone();
        let precedence = Precedence::for_token(&op_token.kind);

        match op_token.kind {
            TokenKind::Plus
            | TokenKind::Minus
            | TokenKind::Star
            | TokenKind::Slash
            | TokenKind::Percent
            | TokenKind::Equal
            | TokenKind::NotEqual
            | TokenKind::Less
            | TokenKind::LessEqual
            | TokenKind::Greater
            | TokenKind::GreaterEqual
            | TokenKind::And
            | TokenKind::Or => {
                self.advance();
                let right = self.parse_expression(precedence)?;
                let end_span = right.span();
                let bop = match op_token.kind {
                    TokenKind::Plus => BinaryOp::Add,
                    TokenKind::Minus => BinaryOp::Sub,
                    TokenKind::Star => BinaryOp::Mul,
                    TokenKind::Slash => BinaryOp::Div,
                    TokenKind::Percent => BinaryOp::Rem,
                    TokenKind::Equal => BinaryOp::Equal,
                    TokenKind::NotEqual => BinaryOp::NotEqual,
                    TokenKind::Less => BinaryOp::Less,
                    TokenKind::LessEqual => BinaryOp::LessEqual,
                    TokenKind::Greater => BinaryOp::Greater,
                    TokenKind::GreaterEqual => BinaryOp::GreaterEqual,
                    TokenKind::And => BinaryOp::And,
                    TokenKind::Or => BinaryOp::Or,
                    _ => unreachable!(),
                };
                Some(Expr::Binary {
                    span: Span::new(
                        left.span().start,
                        end_span.end,
                        left.span().start_loc,
                        end_span.end_loc,
                    ),
                    left: Box::new(left),
                    op: bop,
                    right: Box::new(right),
                })
            }
            TokenKind::NullCoalesce => {
                self.advance();
                let right = self.parse_expression(precedence)?;
                let end_span = right.span();
                Some(Expr::NullCoalesce {
                    span: Span::new(
                        left.span().start,
                        end_span.end,
                        left.span().start_loc,
                        end_span.end_loc,
                    ),
                    left: Box::new(left),
                    right: Box::new(right),
                })
            }
            TokenKind::Dot | TokenKind::OptionalChain => {
                let is_optional = op_token.kind == TokenKind::OptionalChain;
                self.advance();

                let prop = match &self.current_token.kind {
                    TokenKind::Identifier(p) => p.clone(),
                    TokenKind::Spawn => "spawn".to_string(),
                    TokenKind::Move => "move".to_string(),
                    TokenKind::Borrow => "borrow".to_string(),
                    TokenKind::Match => "match".to_string(),
                    TokenKind::Type => "type".to_string(),
                    _ => return None,
                };
                let end_span = self.current_token.span;
                self.advance();

                Some(Expr::MemberAccess {
                    span: Span::new(
                        left.span().start,
                        end_span.end,
                        left.span().start_loc,
                        end_span.end_loc,
                    ),
                    object: Box::new(left),
                    property: prop,
                    is_optional,
                })
            }
            TokenKind::OpenParen => {
                self.advance(); // (
                let mut args = Vec::new();

                while self.current_token.kind != TokenKind::CloseParen
                    && self.current_token.kind != TokenKind::Eof
                {
                    if let Some(arg) = self.parse_expression(Precedence::Lowest) {
                        args.push(arg);
                    } else {
                        self.advance();
                    }
                    if self.current_token.kind == TokenKind::Comma {
                        self.advance();
                    }
                }

                let end_span = self.current_token.span;
                self.expect(TokenKind::CloseParen);

                Some(Expr::Call {
                    span: Span::new(
                        left.span().start,
                        end_span.end,
                        left.span().start_loc,
                        end_span.end_loc,
                    ),
                    callee: Box::new(left),
                    args,
                })
            }
            _ => None,
        }
    }

    pub fn parse_block_expr(&mut self) -> Option<BlockExpr> {
        let start_span = self.current_token.span;
        if !self.expect(TokenKind::OpenBrace) {
            return None;
        }

        // Fast path: if the first content is `ident : ...`, this is an anonymous struct
        if matches!(&self.current_token.kind, TokenKind::Identifier(_))
            && matches!(&self.peek_token.kind, TokenKind::Colon)
        {
            if let Some(s) = self.parse_anonymous_struct_here(start_span) {
                return Some(s);
            }
        }

        let mut statements = Vec::new();
        let mut final_expr: Option<Box<Expr>> = None;

        while self.current_token.kind != TokenKind::CloseBrace
            && self.current_token.kind != TokenKind::Eof
        {
            if let Some(stmt) = self.parse_statement() {
                // If this is a bare expression (no semicolon), it may be the final expression.
                // Don't add it as a statement yet — wait to see if more statements follow.
                if let Stmt::Expr { expr, has_semicolon: false, span: _ } = &stmt {
                    // Peek ahead: if next token is close brace or another non-semicoloned expr,
                    // this is the final expression
                    if self.current_token.kind == TokenKind::CloseBrace
                        || self.current_token.kind == TokenKind::Eof
                    {
                        final_expr = Some(Box::new(expr.clone()));
                        break;
                    }
                    // Don't hold onto it — push as statement, we'll pop later if needed
                    statements.push(stmt);
                } else {
                    statements.push(stmt);
                }
            } else {
                self.advance();
            }
        }

        // If final_expr wasn't set by the peek-ahead, check if last statement
        // is a semicolonless expression that should be promoted
        if final_expr.is_none() {
            if let Some(Stmt::Expr { expr, has_semicolon: false, .. }) = statements.last() {
                let expr = expr.clone();
                statements.pop();
                final_expr = Some(Box::new(expr));
            }
        }

        let end_span = self.current_token.span;
        self.expect(TokenKind::CloseBrace);

        Some(BlockExpr {
            statements,
            final_expr,
            span: Span::new(
                start_span.start,
                end_span.end,
                start_span.start_loc,
                end_span.end_loc,
            ),
        })
    }

    fn parse_statement(&mut self) -> Option<Stmt> {
        match &self.current_token.kind {
            TokenKind::Identifier(name) if self.peek_token.kind == TokenKind::Assign => {
                let start_span = self.current_token.span;
                let target = name.clone();
                self.advance(); // ident
                self.advance(); // =
                let value = self.parse_expression(Precedence::Lowest)?;
                let end_span = value.span();
                if self.current_token.kind == TokenKind::Semicolon {
                    self.advance();
                }
                Some(Stmt::Assign {
                    target,
                    value: Box::new(value),
                    span: Span::new(start_span.start, end_span.end, start_span.start_loc, end_span.end_loc),
                })
            }
            TokenKind::Identifier(name) if self.peek_token.kind == TokenKind::PlusAssign => {
                let start_span = self.current_token.span;
                let target = name.clone();
                self.advance(); // ident
                self.advance(); // +=
                let rhs = self.parse_expression(Precedence::Lowest)?;
                let end_span = rhs.span();
                if self.current_token.kind == TokenKind::Semicolon {
                    self.advance();
                }
                Some(Stmt::Assign {
                    target: target.clone(),
                    value: Box::new(Expr::Binary {
                        left: Box::new(Expr::Identifier { name: target.clone(), span: start_span }),
                        op: BinaryOp::Add,
                        right: Box::new(rhs),
                        span: start_span,
                    }),
                    span: Span::new(start_span.start, end_span.end, start_span.start_loc, end_span.end_loc),
                })
            }
            TokenKind::Identifier(name) if self.peek_token.kind == TokenKind::MinusAssign => {
                let start_span = self.current_token.span;
                let target = name.clone();
                self.advance(); // ident
                self.advance(); // -=
                let rhs = self.parse_expression(Precedence::Lowest)?;
                let end_span = rhs.span();
                if self.current_token.kind == TokenKind::Semicolon {
                    self.advance();
                }
                Some(Stmt::Assign {
                    target: target.clone(),
                    value: Box::new(Expr::Binary {
                        left: Box::new(Expr::Identifier { name: target.clone(), span: start_span }),
                        op: BinaryOp::Sub,
                        right: Box::new(rhs),
                        span: start_span,
                    }),
                    span: Span::new(start_span.start, end_span.end, start_span.start_loc, end_span.end_loc),
                })
            }
            TokenKind::While => {
                let start_span = self.current_token.span;
                self.advance(); // while
                let cond = self.parse_expression(Precedence::Lowest)?;
                let body = self.parse_block_expr()?;
                let end_span = body.span;
                Some(Stmt::Expr {
                    expr: Expr::ForLoop {
                        init: None,
                        cond: Some(Box::new(cond)),
                        update: None,
                        body,
                        span: Span::new(start_span.start, end_span.end, start_span.start_loc, end_span.end_loc),
                    },
                    has_semicolon: false,
                    span: Span::new(start_span.start, end_span.end, start_span.start_loc, end_span.end_loc),
                })
            }
            TokenKind::For => {
                let start_span = self.current_token.span;
                self.advance(); // for

                // for let i = 0; ... (C-style)
                if self.current_token.kind == TokenKind::Let {
                    self.advance(); // let
                    let name = match &self.current_token.kind {
                        TokenKind::Identifier(id) => id.clone(),
                        _ => return None,
                    };
                    let init_span = self.current_token.span;
                    self.advance();
                    self.expect(TokenKind::Assign);
                    let init_val = self.parse_expression(Precedence::Lowest)?;
                    self.expect(TokenKind::Semicolon);
                    let cond = self.parse_expression(Precedence::Lowest)?;
                    self.expect(TokenKind::Semicolon);
                    // Parse update: handle i += 1, i -= 1, i = expr, or bare expr
                    let update_var = match &self.current_token.kind {
                        TokenKind::Identifier(id) => id.clone(),
                        _ => String::new(),
                    };
                    let update_stmt = if !update_var.is_empty() {
                        let u_span = self.current_token.span;
                        self.advance();
                        match &self.current_token.kind {
                            TokenKind::PlusAssign => {
                                self.advance();
                                let rhs = self.parse_expression(Precedence::Lowest)?;
                                Some(Stmt::Assign {
                                    target: update_var.clone(),
                                    value: Box::new(Expr::Binary {
                                        left: Box::new(Expr::Identifier { name: update_var.clone(), span: u_span }),
                                        op: BinaryOp::Add,
                                        right: Box::new(rhs),
                                        span: u_span,
                                    }),
                                    span: u_span,
                                })
                            }
                            TokenKind::MinusAssign => {
                                self.advance();
                                let rhs = self.parse_expression(Precedence::Lowest)?;
                                Some(Stmt::Assign {
                                    target: update_var.clone(),
                                    value: Box::new(Expr::Binary {
                                        left: Box::new(Expr::Identifier { name: update_var.clone(), span: u_span }),
                                        op: BinaryOp::Sub,
                                        right: Box::new(rhs),
                                        span: u_span,
                                    }),
                                    span: u_span,
                                })
                            }
                            TokenKind::Assign => {
                                self.advance();
                                let rhs = self.parse_expression(Precedence::Lowest)?;
                                Some(Stmt::Assign {
                                    target: update_var.clone(),
                                    value: Box::new(rhs),
                                    span: u_span,
                                })
                            }
                            _ => None,
                        }
                    } else {
                        None
                    };
                    let body = self.parse_block_expr()?;
                    let end_span = body.span;

                    let init_stmt = Stmt::VarDecl {
                        is_const: false,
                        name,
                        type_ann: None,
                        init: Some(init_val),
                        span: init_span,
                    };

                    return Some(Stmt::Expr {
                        expr: Expr::ForLoop {
                            init: Some(Box::new(init_stmt)),
                            cond: Some(Box::new(cond)),
                            update: update_stmt.map(Box::new),
                            body,
                            span: Span::new(start_span.start, end_span.end, start_span.start_loc, end_span.end_loc),
                        },
                        has_semicolon: false,
                        span: Span::new(start_span.start, end_span.end, start_span.start_loc, end_span.end_loc),
                    });
                }

                // for ident in expr { ... } (foreach) or for ident, ident in expr { ... } (enumerate)
                let first = match &self.current_token.kind {
                    TokenKind::Identifier(id) => id.clone(),
                    _ => return None,
                };
                self.advance();

                let (index_var, item_var) = if self.current_token.kind == TokenKind::Comma {
                    self.advance();
                    let second = match &self.current_token.kind {
                        TokenKind::Identifier(id) => id.clone(),
                        _ => return None,
                    };
                    self.advance();
                    (Some(first), second)
                } else {
                    (None, first)
                };

                if self.current_token.kind != TokenKind::Identifier("in".into()) {
                    self.diagnostics.push(Diagnostic::error("Expected 'in' in for loop").with_span(self.current_token.span));
                    return None;
                }
                self.advance(); // in

                let iterable = self.parse_expression(Precedence::Lowest)?;
                let body = self.parse_block_expr()?;
                let end_span = body.span;

                return Some(Stmt::Expr {
                    expr: Expr::ForIn {
                        index_var,
                        item_var,
                        iterable: Box::new(iterable),
                        body,
                        span: Span::new(start_span.start, end_span.end, start_span.start_loc, end_span.end_loc),
                    },
                    has_semicolon: false,
                    span: Span::new(start_span.start, end_span.end, start_span.start_loc, end_span.end_loc),
                });
            }
            TokenKind::Let | TokenKind::Const => {
                let is_const = self.current_token.kind == TokenKind::Const;
                let start_span = self.current_token.span;
                self.advance();

                let name = match &self.current_token.kind {
                    TokenKind::Identifier(id) => id.clone(),
                    _ => return None,
                };
                self.advance();

                // Check for struct destructuring: `let Point { x, y } = expr`
                if self.current_token.kind == TokenKind::OpenBrace
                    && name.chars().next().map_or(false, |c| c.is_uppercase())
                {
                    self.advance(); // {
                    let mut fields = Vec::new();
                    while self.current_token.kind != TokenKind::CloseBrace
                        && self.current_token.kind != TokenKind::Eof
                    {
                        if let TokenKind::Identifier(fname) = &self.current_token.kind {
                            fields.push(fname.clone());
                            self.advance();
                        }
                        if self.current_token.kind == TokenKind::Comma {
                            self.advance();
                        }
                    }
                    self.expect(TokenKind::CloseBrace);
                    self.expect(TokenKind::Assign);
                    let init = self.parse_expression(Precedence::Lowest)?;
                    if self.current_token.kind == TokenKind::Semicolon {
                        self.advance();
                    }
                    let end_span = self.current_token.span;
                    return Some(Stmt::Destructure {
                        struct_name: name,
                        fields,
                        init: Box::new(init),
                        span: Span::new(
                            start_span.start,
                            end_span.end,
                            start_span.start_loc,
                            end_span.end_loc,
                        ),
                    });
                }

                let type_ann = if self.current_token.kind == TokenKind::Colon {
                    self.advance();
                    self.parse_type_annotation()
                } else {
                    None
                };

                let init = if self.current_token.kind == TokenKind::Assign {
                    self.advance();
                    self.parse_expression(Precedence::Lowest)
                } else {
                    None
                };

                let end_span = self.current_token.span;
                if self.current_token.kind == TokenKind::Semicolon {
                    self.advance();
                }

                Some(Stmt::VarDecl {
                    is_const,
                    name,
                    type_ann,
                    init,
                    span: Span::new(
                        start_span.start,
                        end_span.end,
                        start_span.start_loc,
                        end_span.end_loc,
                    ),
                })
            }
            TokenKind::Return => {
                let start_span = self.current_token.span;
                self.advance();

                let value = if self.current_token.kind != TokenKind::Semicolon
                    && self.current_token.kind != TokenKind::CloseBrace
                {
                    self.parse_expression(Precedence::Lowest)
                } else {
                    None
                };

                let end_span = self.current_token.span;
                if self.current_token.kind == TokenKind::Semicolon {
                    self.advance();
                }

                Some(Stmt::Return {
                    value,
                    span: Span::new(
                        start_span.start,
                        end_span.end,
                        start_span.start_loc,
                        end_span.end_loc,
                    ),
                })
            }
            TokenKind::Defer => {
                let start_span = self.current_token.span;
                self.advance();

                let body = self.parse_expression(Precedence::Lowest)?;
                let end_span = body.span();

                if self.current_token.kind == TokenKind::Semicolon {
                    self.advance();
                }

                Some(Stmt::Defer {
                    body: Box::new(body),
                    span: Span::new(
                        start_span.start,
                        end_span.end,
                        start_span.start_loc,
                        end_span.end_loc,
                    ),
                })
            }
            TokenKind::Break => {
                let span = self.current_token.span;
                self.advance();
                if self.current_token.kind == TokenKind::Semicolon {
                    self.advance();
                }
                Some(Stmt::Break { span })
            }
            TokenKind::Continue => {
                let span = self.current_token.span;
                self.advance();
                if self.current_token.kind == TokenKind::Semicolon {
                    self.advance();
                }
                Some(Stmt::Continue { span })
            }
            _ => {
                let expr = self.parse_expression(Precedence::Lowest)?;
                let has_semicolon = if self.current_token.kind == TokenKind::Semicolon {
                    self.advance();
                    true
                } else {
                    false
                };
                let span = expr.span();
                Some(Stmt::Expr {
                    expr,
                    has_semicolon,
                    span,
                })
            }
        }
    }

    fn recover(&mut self) {
        while self.current_token.kind != TokenKind::Eof {
            match self.current_token.kind {
                TokenKind::Semicolon | TokenKind::CloseBrace => {
                    self.advance();
                    break;
                }
                TokenKind::Struct
                | TokenKind::Extend
                | TokenKind::Enum
                | TokenKind::ErrorKw
                | TokenKind::Actor
                | TokenKind::Fn
                | TokenKind::Import
                | TokenKind::Export
                | TokenKind::Spawn
                | TokenKind::Group => {
                    break;
                }
                _ => self.advance(),
            }
        }
    }
}

fn inner_decl_span(decl: &Decl) -> Span {
    match decl {
        Decl::Struct { span, .. }
        | Decl::Extend { span, .. }
        | Decl::Enum { span, .. }
        | Decl::Capability { span, .. }
        | Decl::Import { span, .. }
        | Decl::Export { span, .. }
        | Decl::Extern { span, .. } => *span,
        Decl::Fn(f) => f.span,
    }
}
