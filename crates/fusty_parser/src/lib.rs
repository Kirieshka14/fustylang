pub mod ast;

use ast::*;
use fusty_lexer::token::{Span, Token, TokenKind};

pub struct Parser {
    tokens: Vec<Token>,
    cursor: usize,
}

#[derive(Debug)]
pub struct ParseError {
    pub message: String,
    pub span: Span,
}

impl Parser {
    pub fn new(mut tokens: Vec<Token>) -> Self {
        if tokens.is_empty() {
            tokens.push(Token::new(TokenKind::Eof, Span::new(0, 0, 1, 1)));
        }
        Self { tokens, cursor: 0 }
    }

    fn peek(&self) -> &Token {
        self.tokens
            .get(self.cursor)
            .unwrap_or_else(|| self.tokens.last().expect("Tokens should not be empty"))
    }

    fn peek_kind(&self) -> &TokenKind {
        &self.peek().kind
    }

    fn peek_next_kind(&self) -> Option<&TokenKind> {
        self.tokens.get(self.cursor + 1).map(|t| &t.kind)
    }

    fn advance(&mut self) -> Token {
        if self.cursor < self.tokens.len() {
            let tok = self.tokens[self.cursor].clone();
            self.cursor += 1;
            tok
        } else {
            self.tokens.last().cloned().unwrap()
        }
    }

    fn check(&self, kind: &TokenKind) -> bool {
        self.peek_kind() == kind
    }

    fn match_token(&mut self, kind: &TokenKind) -> bool {
        if self.check(kind) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, kind: &TokenKind, msg: &str) -> Result<Token, ParseError> {
        if self.check(kind) {
            Ok(self.advance())
        } else {
            let tok = self.peek();
            Err(ParseError {
                message: format!(
                    "B-baka! I expected {}, not {:?}! It's not like I'm going to fix it for you... >///<",
                    msg, tok.kind
                ),
                span: tok.span,
            })
        }
    }

    pub fn parse_program(&mut self) -> Result<Program, ParseError> {
        let mut items = Vec::new();
        while !self.check(&TokenKind::Eof) {
            items.push(self.parse_item()?);
        }
        Ok(Program { items })
    }

    pub fn parse_item(&mut self) -> Result<Item, ParseError> {
        if self.check(&TokenKind::Meow) && self.peek_next_kind() == Some(&TokenKind::Nya) {
            self.advance(); // consume `meow`
            let mut func = self.parse_fn()?;
            func.is_meow = true;
            return Ok(Item::Fn(func));
        }

        match self.peek_kind() {
            TokenKind::Nya => Ok(Item::Fn(self.parse_fn()?)),
            TokenKind::Struct => Ok(Item::Struct(self.parse_struct()?)),
            TokenKind::Enum => Ok(Item::Enum(self.parse_enum()?)),
            TokenKind::Impl => Ok(Item::Impl(self.parse_impl()?)),
            _ => Ok(Item::Stmt(self.parse_stmt()?)),
        }
    }

    fn parse_fn(&mut self) -> Result<FnDecl, ParseError> {
        let start = self.expect(&TokenKind::Nya, "'nya'")?.span;
        let name =
            match self.peek_kind() {
                TokenKind::Ident(_) => {
                    if let TokenKind::Ident(s) = self.advance().kind {
                        s
                    } else {
                        unreachable!()
                    }
                }
                _ => return Err(ParseError {
                    message:
                        "B-baka! You forgot to name your 'nya' function! Don't make me guess! >///<"
                            .into(),
                    span: self.peek().span,
                }),
            };

        self.expect(&TokenKind::LParen, "'('")?;
        let mut params = Vec::new();
        while !self.check(&TokenKind::RParen) && !self.check(&TokenKind::Eof) {
            let mut is_mut = false;
            if self.match_token(&TokenKind::Mut) {
                is_mut = true;
            }

            let param_tok = self.advance();
            let param_name =
                match param_tok.kind {
                    TokenKind::SelfValue => "self".to_string(),
                    TokenKind::Ident(s) => s,
                    _ => return Err(ParseError {
                        message:
                            "B-baka! Expected a parameter name or 'self', not whatever that is! >.<"
                                .into(),
                        span: param_tok.span,
                    }),
                };

            let ty = if self.match_token(&TokenKind::Colon) {
                Some(self.parse_type()?)
            } else {
                None
            };

            params.push(Param {
                name: param_name,
                is_mut,
                ty,
                span: param_tok.span,
            });

            if !self.match_token(&TokenKind::Comma) {
                break;
            }
        }
        self.expect(&TokenKind::RParen, "')'")?;

        let return_ty = if self.match_token(&TokenKind::Arrow) {
            Some(self.parse_type()?)
        } else {
            None
        };

        let body = self.parse_block()?;
        let span = Span::new(start.start, body.span.end, start.line, start.col);

        Ok(FnDecl {
            name,
            is_meow: false,
            params,
            return_ty,
            body,
            span,
        })
    }

    fn parse_struct(&mut self) -> Result<StructDecl, ParseError> {
        let start = self.expect(&TokenKind::Struct, "'struct'")?.span;
        let name = match self.advance().kind {
            TokenKind::Ident(s) => s,
            _ => {
                return Err(ParseError {
                    message: "Expected struct name".into(),
                    span: start,
                })
            }
        };

        self.expect(&TokenKind::LBrace, "'{'")?;
        let mut fields = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.check(&TokenKind::Eof) {
            let mut is_mut = false;
            if self.match_token(&TokenKind::Mut) {
                is_mut = true;
            }
            let field_tok = self.advance();
            let field_name = match field_tok.kind {
                TokenKind::Ident(s) => s,
                _ => {
                    return Err(ParseError {
                        message: "Expected field name".into(),
                        span: field_tok.span,
                    })
                }
            };

            self.expect(&TokenKind::Colon, "':'")?;
            let ty = self.parse_type()?;
            fields.push(StructField {
                name: field_name,
                is_mut,
                ty,
                span: field_tok.span,
            });

            self.match_token(&TokenKind::Comma);
        }
        let end = self.expect(&TokenKind::RBrace, "'}'")?.span;

        Ok(StructDecl {
            name,
            fields,
            span: Span::new(start.start, end.end, start.line, start.col),
        })
    }

    fn parse_enum(&mut self) -> Result<EnumDecl, ParseError> {
        let start = self.expect(&TokenKind::Enum, "'enum'")?.span;
        let name = match self.advance().kind {
            TokenKind::Ident(s) => s,
            _ => {
                return Err(ParseError {
                    message: "Expected enum name".into(),
                    span: start,
                })
            }
        };

        self.expect(&TokenKind::LBrace, "'{'")?;
        let mut variants = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.check(&TokenKind::Eof) {
            let var_tok = self.advance();
            let var_name = match var_tok.kind {
                TokenKind::Ident(s) => s,
                _ => {
                    return Err(ParseError {
                        message: "Expected enum variant name".into(),
                        span: var_tok.span,
                    })
                }
            };

            let mut fields = Vec::new();
            if self.match_token(&TokenKind::LParen) {
                while !self.check(&TokenKind::RParen) && !self.check(&TokenKind::Eof) {
                    fields.push(self.parse_type()?);
                    if !self.match_token(&TokenKind::Comma) {
                        break;
                    }
                }
                self.expect(&TokenKind::RParen, "')'")?;
            }

            variants.push(EnumVariant {
                name: var_name,
                fields,
                span: var_tok.span,
            });

            self.match_token(&TokenKind::Comma);
        }
        let end = self.expect(&TokenKind::RBrace, "'}'")?.span;

        Ok(EnumDecl {
            name,
            variants,
            span: Span::new(start.start, end.end, start.line, start.col),
        })
    }

    fn parse_impl(&mut self) -> Result<ImplBlock, ParseError> {
        let start = self.expect(&TokenKind::Impl, "'impl'")?.span;
        let target = match self.advance().kind {
            TokenKind::Ident(s) => s,
            _ => {
                return Err(ParseError {
                    message: "Expected type name for impl".into(),
                    span: start,
                })
            }
        };

        self.expect(&TokenKind::LBrace, "'{'")?;
        let mut methods = Vec::new();
        while !self.check(&TokenKind::RBrace) && !self.check(&TokenKind::Eof) {
            if self.check(&TokenKind::Meow) && self.peek_next_kind() == Some(&TokenKind::Nya) {
                self.advance();
                let mut func = self.parse_fn()?;
                func.is_meow = true;
                methods.push(func);
            } else {
                methods.push(self.parse_fn()?);
            }
        }
        let end = self.expect(&TokenKind::RBrace, "'}'")?.span;

        Ok(ImplBlock {
            target,
            methods,
            span: Span::new(start.start, end.end, start.line, start.col),
        })
    }

    fn parse_type(&mut self) -> Result<Type, ParseError> {
        let tok = self.advance();
        match tok.kind {
            TokenKind::Vec2 => Ok(Type::Vec2),
            TokenKind::Vec3 => Ok(Type::Vec3),
            TokenKind::Ident(ref s) => match s.as_str() {
                "i32" => Ok(Type::I32),
                "i64" => Ok(Type::I64),
                "f32" => Ok(Type::F32),
                "f64" => Ok(Type::F64),
                "bool" => Ok(Type::Bool),
                "string" => Ok(Type::String),
                other => Ok(Type::Custom(other.to_string())),
            },
            _ => Err(ParseError {
                message: format!("Expected type, found {:?}", tok.kind),
                span: tok.span,
            }),
        }
    }

    pub fn parse_stmt(&mut self) -> Result<Stmt, ParseError> {
        match self.peek_kind() {
            TokenKind::Let => self.parse_let_stmt(),
            TokenKind::Return => self.parse_return_stmt(),
            TokenKind::Break => self.parse_break_stmt(),
            TokenKind::Continue => self.parse_continue_stmt(),
            TokenKind::Yield => self.parse_yield_stmt(),
            TokenKind::While => self.parse_while_stmt(),
            TokenKind::Wait => self.parse_wait_stmt(),
            _ => {
                let expr = self.parse_expr()?;
                self.finish_expr_or_assign_stmt(expr)
            }
        }
    }

    fn finish_expr_or_assign_stmt(&mut self, expr: Expr) -> Result<Stmt, ParseError> {
        // Check for assignment: target = val; or target += val;
        let assign_op = if self.match_token(&TokenKind::Assign) {
            Some(None)
        } else if self.match_token(&TokenKind::PlusAssign) {
            Some(Some(BinOp::Add))
        } else if self.match_token(&TokenKind::MinusAssign) {
            Some(Some(BinOp::Sub))
        } else if self.match_token(&TokenKind::StarAssign) {
            Some(Some(BinOp::Mul))
        } else if self.match_token(&TokenKind::SlashAssign) {
            Some(Some(BinOp::Div))
        } else {
            None
        };

        if let Some(compound_op) = assign_op {
            let val = self.parse_expr()?;
            let end = self.expect(&TokenKind::Semicolon, "';'")?.span;
            let span = Span::new(
                expr.span().start,
                end.end,
                expr.span().line,
                expr.span().col,
            );

            if let Some(op) = compound_op {
                Ok(Stmt::CompoundAssign {
                    target: expr,
                    op,
                    value: val,
                    span,
                })
            } else {
                Ok(Stmt::Assign {
                    target: expr,
                    value: val,
                    span,
                })
            }
        } else {
            self.match_token(&TokenKind::Semicolon);
            Ok(Stmt::Expr(expr))
        }
    }

    fn parse_break_stmt(&mut self) -> Result<Stmt, ParseError> {
        let start = self.expect(&TokenKind::Break, "'break'")?.span;
        let end = self.expect(&TokenKind::Semicolon, "';'")?.span;
        Ok(Stmt::Break(Span::new(
            start.start,
            end.end,
            start.line,
            start.col,
        )))
    }

    fn parse_continue_stmt(&mut self) -> Result<Stmt, ParseError> {
        let start = self.expect(&TokenKind::Continue, "'continue'")?.span;
        let end = self.expect(&TokenKind::Semicolon, "';'")?.span;
        Ok(Stmt::Continue(Span::new(
            start.start,
            end.end,
            start.line,
            start.col,
        )))
    }

    fn parse_yield_stmt(&mut self) -> Result<Stmt, ParseError> {
        let start = self.expect(&TokenKind::Yield, "'yield'")?.span;
        let end = self.expect(&TokenKind::Semicolon, "';'")?.span;
        Ok(Stmt::Yield(Span::new(
            start.start,
            end.end,
            start.line,
            start.col,
        )))
    }

    fn parse_let_stmt(&mut self) -> Result<Stmt, ParseError> {
        let start = self.expect(&TokenKind::Let, "'let'")?.span;
        let mut is_mut = false;
        if self.match_token(&TokenKind::Mut) {
            is_mut = true;
        }

        let name_tok = self.advance();
        let name = match name_tok.kind {
            TokenKind::Ident(s) => s,
            _ => {
                return Err(ParseError {
                    message: "Expected variable name after let".into(),
                    span: name_tok.span,
                })
            }
        };

        let ty = if self.match_token(&TokenKind::Colon) {
            Some(self.parse_type()?)
        } else {
            None
        };

        let init = if self.match_token(&TokenKind::Assign) {
            Some(self.parse_expr()?)
        } else {
            None
        };

        let end = self.expect(&TokenKind::Semicolon, "';'")?.span;
        let span = Span::new(start.start, end.end, start.line, start.col);

        Ok(Stmt::Let {
            name,
            is_mut,
            ty,
            init,
            span,
        })
    }

    fn parse_return_stmt(&mut self) -> Result<Stmt, ParseError> {
        let start = self.expect(&TokenKind::Return, "'return'")?.span;
        let val = if !self.check(&TokenKind::Semicolon) {
            Some(self.parse_expr()?)
        } else {
            None
        };
        let end = self.expect(&TokenKind::Semicolon, "';'")?.span;
        Ok(Stmt::Return(
            val,
            Span::new(start.start, end.end, start.line, start.col),
        ))
    }

    fn parse_while_stmt(&mut self) -> Result<Stmt, ParseError> {
        let start = self.expect(&TokenKind::While, "'while'")?.span;
        let cond = self.parse_expr()?;
        let body = self.parse_block()?;
        let span = Span::new(start.start, body.span.end, start.line, start.col);
        Ok(Stmt::While { cond, body, span })
    }

    fn parse_wait_stmt(&mut self) -> Result<Stmt, ParseError> {
        let start = self.expect(&TokenKind::Wait, "'wait'")?.span;
        let duration = self.parse_expr()?;
        let end = self.expect(&TokenKind::Semicolon, "';'")?.span;
        let span = Span::new(start.start, end.end, start.line, start.col);
        Ok(Stmt::Expr(Expr::Wait {
            duration: Box::new(duration),
            span,
        }))
    }

    pub fn parse_block(&mut self) -> Result<Block, ParseError> {
        let start = self.expect(&TokenKind::LBrace, "'{'")?.span;
        let mut stmts = Vec::new();
        let mut final_expr = None;

        while !self.check(&TokenKind::RBrace) && !self.check(&TokenKind::Eof) {
            match self.peek_kind() {
                TokenKind::Let
                | TokenKind::Return
                | TokenKind::Break
                | TokenKind::Continue
                | TokenKind::Yield
                | TokenKind::While
                | TokenKind::Wait => {
                    stmts.push(self.parse_stmt()?);
                }
                _ => {
                    let expr = self.parse_expr()?;
                    let is_assign = matches!(
                        self.peek_kind(),
                        TokenKind::Assign
                            | TokenKind::PlusAssign
                            | TokenKind::MinusAssign
                            | TokenKind::StarAssign
                            | TokenKind::SlashAssign
                    );
                    if is_assign {
                        stmts.push(self.finish_expr_or_assign_stmt(expr)?);
                    } else if self.match_token(&TokenKind::Semicolon) {
                        stmts.push(Stmt::Expr(expr));
                    } else if self.check(&TokenKind::RBrace) {
                        final_expr = Some(Box::new(expr));
                        break;
                    } else {
                        stmts.push(Stmt::Expr(expr));
                    }
                }
            }
        }

        let end = self.expect(&TokenKind::RBrace, "'}'")?.span;
        Ok(Block {
            stmts,
            expr: final_expr,
            span: Span::new(start.start, end.end, start.line, start.col),
        })
    }

    pub fn parse_expr(&mut self) -> Result<Expr, ParseError> {
        self.parse_or()
    }

    fn parse_or(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_and()?;
        while self.match_token(&TokenKind::Or) {
            let right = self.parse_and()?;
            let span = Span::new(
                left.span().start,
                right.span().end,
                left.span().line,
                left.span().col,
            );
            left = Expr::Binary {
                left: Box::new(left),
                op: BinOp::Or,
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn parse_and(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_equality()?;
        while self.match_token(&TokenKind::And) {
            let right = self.parse_equality()?;
            let span = Span::new(
                left.span().start,
                right.span().end,
                left.span().line,
                left.span().col,
            );
            left = Expr::Binary {
                left: Box::new(left),
                op: BinOp::And,
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn parse_equality(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_comparison()?;
        loop {
            let op = if self.match_token(&TokenKind::Eq) {
                BinOp::Eq
            } else if self.match_token(&TokenKind::NotEq) {
                BinOp::NotEq
            } else {
                break;
            };
            let right = self.parse_comparison()?;
            let span = Span::new(
                left.span().start,
                right.span().end,
                left.span().line,
                left.span().col,
            );
            left = Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn parse_comparison(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_uwu()?;
        loop {
            let op = if self.match_token(&TokenKind::Lt) {
                BinOp::Lt
            } else if self.match_token(&TokenKind::LtEq) {
                BinOp::LtEq
            } else if self.match_token(&TokenKind::Gt) {
                BinOp::Gt
            } else if self.match_token(&TokenKind::GtEq) {
                BinOp::GtEq
            } else {
                break;
            };
            let right = self.parse_uwu()?;
            let span = Span::new(
                left.span().start,
                right.span().end,
                left.span().line,
                left.span().col,
            );
            left = Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    /// `a uwu b` — the cuddle operator. Binds tighter than comparisons,
    /// looser than `+`/`-`, so `a + b uwu c + d` = `(a + b) uwu (c + d)`.
    fn parse_uwu(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_term()?;
        while self.match_token(&TokenKind::Uwu) {
            let right = self.parse_term()?;
            let span = Span::new(
                left.span().start,
                right.span().end,
                left.span().line,
                left.span().col,
            );
            left = Expr::Binary {
                left: Box::new(left),
                op: BinOp::Uwu,
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn parse_term(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_factor()?;
        loop {
            let op = if self.match_token(&TokenKind::Plus) {
                BinOp::Add
            } else if self.match_token(&TokenKind::Minus) {
                BinOp::Sub
            } else {
                break;
            };
            let right = self.parse_factor()?;
            let span = Span::new(
                left.span().start,
                right.span().end,
                left.span().line,
                left.span().col,
            );
            left = Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn parse_factor(&mut self) -> Result<Expr, ParseError> {
        let mut left = self.parse_unary()?;
        loop {
            let op = if self.match_token(&TokenKind::Star) {
                BinOp::Mul
            } else if self.match_token(&TokenKind::Slash) {
                BinOp::Div
            } else if self.match_token(&TokenKind::Percent) {
                BinOp::Mod
            } else {
                break;
            };
            let right = self.parse_unary()?;
            let span = Span::new(
                left.span().start,
                right.span().end,
                left.span().line,
                left.span().col,
            );
            left = Expr::Binary {
                left: Box::new(left),
                op,
                right: Box::new(right),
                span,
            };
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, ParseError> {
        if self.check(&TokenKind::Bang) {
            let start = self.advance().span;
            let expr = self.parse_unary()?;
            let span = Span::new(start.start, expr.span().end, start.line, start.col);
            return Ok(Expr::Unary {
                op: UnOp::Not,
                expr: Box::new(expr),
                span,
            });
        }
        if self.check(&TokenKind::Minus) {
            let start = self.advance().span;
            let expr = self.parse_unary()?;
            let span = Span::new(start.start, expr.span().end, start.line, start.col);
            return Ok(Expr::Unary {
                op: UnOp::Neg,
                expr: Box::new(expr),
                span,
            });
        }
        self.parse_postfix()
    }

    fn parse_postfix(&mut self) -> Result<Expr, ParseError> {
        let mut expr = self.parse_primary()?;

        loop {
            if self.match_token(&TokenKind::LParen) {
                // Call
                let mut args = Vec::new();
                while !self.check(&TokenKind::RParen) && !self.check(&TokenKind::Eof) {
                    args.push(self.parse_expr()?);
                    if !self.match_token(&TokenKind::Comma) {
                        break;
                    }
                }
                let end = self.expect(&TokenKind::RParen, "')'")?.span;
                let span = Span::new(
                    expr.span().start,
                    end.end,
                    expr.span().line,
                    expr.span().col,
                );
                expr = Expr::Call {
                    callee: Box::new(expr),
                    args,
                    span,
                };
            } else if self.match_token(&TokenKind::Dot) {
                let member_tok = self.advance();
                let member_name = match member_tok.kind {
                    TokenKind::Ident(s) => s,
                    _ => {
                        return Err(ParseError {
                            message: "Expected member name after '.'".into(),
                            span: member_tok.span,
                        })
                    }
                };

                if self.match_token(&TokenKind::LParen) {
                    // Method call
                    let mut args = Vec::new();
                    while !self.check(&TokenKind::RParen) && !self.check(&TokenKind::Eof) {
                        args.push(self.parse_expr()?);
                        if !self.match_token(&TokenKind::Comma) {
                            break;
                        }
                    }
                    let end = self.expect(&TokenKind::RParen, "')'")?.span;
                    let span = Span::new(
                        expr.span().start,
                        end.end,
                        expr.span().line,
                        expr.span().col,
                    );
                    expr = Expr::MethodCall {
                        target: Box::new(expr),
                        method: member_name,
                        args,
                        span,
                    };
                } else {
                    // Field access
                    let span = Span::new(
                        expr.span().start,
                        member_tok.span.end,
                        expr.span().line,
                        expr.span().col,
                    );
                    expr = Expr::FieldAccess {
                        target: Box::new(expr),
                        field: member_name,
                        span,
                    };
                }
            } else {
                break;
            }
        }

        Ok(expr)
    }

    fn parse_primary(&mut self) -> Result<Expr, ParseError> {
        let tok = self.advance();
        match tok.kind {
            TokenKind::IntLit(val) => Ok(Expr::Literal(Literal::Int(val), tok.span)),
            TokenKind::FloatLit(val) => Ok(Expr::Literal(Literal::Float(val), tok.span)),
            TokenKind::StringLit(val) => Ok(Expr::Literal(Literal::String(val), tok.span)),
            TokenKind::True => Ok(Expr::Literal(Literal::Bool(true), tok.span)),
            TokenKind::False => Ok(Expr::Literal(Literal::Bool(false), tok.span)),
            TokenKind::Ident(s) => Ok(Expr::Ident(s, tok.span)),
            TokenKind::SelfValue => Ok(Expr::Ident("self".to_string(), tok.span)),

            TokenKind::Vec2 => {
                self.expect(&TokenKind::LParen, "'('")?;
                let x = self.parse_expr()?;
                self.expect(&TokenKind::Comma, "','")?;
                let y = self.parse_expr()?;
                let end = self.expect(&TokenKind::RParen, "')'")?.span;
                let span = Span::new(tok.span.start, end.end, tok.span.line, tok.span.col);
                Ok(Expr::Vec2 {
                    x: Box::new(x),
                    y: Box::new(y),
                    span,
                })
            }

            TokenKind::Vec3 => {
                self.expect(&TokenKind::LParen, "'('")?;
                let x = self.parse_expr()?;
                self.expect(&TokenKind::Comma, "','")?;
                let y = self.parse_expr()?;
                self.expect(&TokenKind::Comma, "','")?;
                let z = self.parse_expr()?;
                let end = self.expect(&TokenKind::RParen, "')'")?.span;
                let span = Span::new(tok.span.start, end.end, tok.span.line, tok.span.col);
                Ok(Expr::Vec3 {
                    x: Box::new(x),
                    y: Box::new(y),
                    z: Box::new(z),
                    span,
                })
            }

            TokenKind::Meow => {
                // `meow { ... }` block!
                let block = self.parse_block()?;
                let span = Span::new(tok.span.start, block.span.end, tok.span.line, tok.span.col);
                Ok(Expr::MeowBlock { block, span })
            }

            TokenKind::Baka => {
                // `baka!("message")` or `baka!()`
                self.expect(&TokenKind::Bang, "'!' after 'baka'")?;
                self.expect(&TokenKind::LParen, "'(' after 'baka!'")?;
                let message = if !self.check(&TokenKind::RParen) && !self.check(&TokenKind::Eof) {
                    Some(Box::new(self.parse_expr()?))
                } else {
                    None
                };
                let end = self.expect(&TokenKind::RParen, "')'")?.span;
                let span = Span::new(tok.span.start, end.end, tok.span.line, tok.span.col);
                Ok(Expr::Baka { message, span })
            }

            TokenKind::If => {
                let cond = self.parse_expr()?;
                let then_branch = self.parse_block()?;
                let else_branch = if self.match_token(&TokenKind::Else) {
                    if self.check(&TokenKind::If) {
                        let elif_expr = self.parse_primary()?;
                        let elif_span = elif_expr.span();
                        Some(Block {
                            stmts: Vec::new(),
                            expr: Some(Box::new(elif_expr)),
                            span: elif_span,
                        })
                    } else {
                        Some(self.parse_block()?)
                    }
                } else {
                    None
                };
                let end_span = else_branch
                    .as_ref()
                    .map(|b| b.span)
                    .unwrap_or(then_branch.span);
                let span = Span::new(tok.span.start, end_span.end, tok.span.line, tok.span.col);
                Ok(Expr::If {
                    cond: Box::new(cond),
                    then_branch,
                    else_branch,
                    span,
                })
            }

            TokenKind::Match => {
                let value = self.parse_expr()?;
                self.expect(&TokenKind::LBrace, "'{'")?;
                let mut arms = Vec::new();

                while !self.check(&TokenKind::RBrace) && !self.check(&TokenKind::Eof) {
                    let pat_start = self.peek().span;
                    let pattern = self.parse_pattern()?;
                    self.expect(&TokenKind::FatArrow, "'=>'")?;
                    let body = self.parse_expr()?;
                    let arm_end = body.span().end;
                    self.match_token(&TokenKind::Comma);

                    arms.push(MatchArm {
                        pattern,
                        body,
                        span: Span::new(pat_start.start, arm_end, pat_start.line, pat_start.col),
                    });
                }

                let end = self.expect(&TokenKind::RBrace, "'}'")?.span;
                let span = Span::new(tok.span.start, end.end, tok.span.line, tok.span.col);
                Ok(Expr::Match {
                    value: Box::new(value),
                    arms,
                    span,
                })
            }

            TokenKind::LParen => {
                let expr = self.parse_expr()?;
                self.expect(&TokenKind::RParen, "')'")?;
                Ok(expr)
            }

            TokenKind::LBrace => {
                // Standalone block expression
                self.cursor -= 1; // back up to parse block properly
                let block = self.parse_block()?;
                Ok(Expr::Block(block))
            }

            _ => Err(ParseError {
                message: format!("Unexpected token in expression: {:?}", tok.kind),
                span: tok.span,
            }),
        }
    }

    fn parse_pattern(&mut self) -> Result<Pattern, ParseError> {
        let tok = self.advance();
        match tok.kind {
            TokenKind::Underscore => Ok(Pattern::Wildcard),
            TokenKind::Minus => {
                let next = self.advance();
                match next.kind {
                    TokenKind::IntLit(v) => Ok(Pattern::Literal(Literal::Int(-v))),
                    TokenKind::FloatLit(v) => Ok(Pattern::Literal(Literal::Float(-v))),
                    _ => Err(ParseError {
                        message: format!(
                            "Expected number after '-' in pattern, found {:?}",
                            next.kind
                        ),
                        span: next.span,
                    }),
                }
            }
            TokenKind::IntLit(v) => Ok(Pattern::Literal(Literal::Int(v))),
            TokenKind::FloatLit(v) => Ok(Pattern::Literal(Literal::Float(v))),
            TokenKind::StringLit(v) => Ok(Pattern::Literal(Literal::String(v))),
            TokenKind::True => Ok(Pattern::Literal(Literal::Bool(true))),
            TokenKind::False => Ok(Pattern::Literal(Literal::Bool(false))),
            TokenKind::Ident(ident) => {
                if self.match_token(&TokenKind::DoubleColon) {
                    let var_tok = self.advance();
                    let var_name = match var_tok.kind {
                        TokenKind::Ident(s) => s,
                        _ => {
                            return Err(ParseError {
                                message: "Expected variant name".into(),
                                span: var_tok.span,
                            })
                        }
                    };

                    let mut bindings = Vec::new();
                    if self.match_token(&TokenKind::LParen) {
                        while !self.check(&TokenKind::RParen) && !self.check(&TokenKind::Eof) {
                            let b_tok = self.advance();
                            if let TokenKind::Ident(b) = b_tok.kind {
                                bindings.push(b);
                            }
                            if !self.match_token(&TokenKind::Comma) {
                                break;
                            }
                        }
                        self.expect(&TokenKind::RParen, "')'")?;
                    }

                    Ok(Pattern::Variant {
                        enum_name: Some(ident),
                        variant_name: var_name,
                        bindings,
                    })
                } else {
                    Ok(Pattern::Ident(ident))
                }
            }
            _ => Err(ParseError {
                message: format!("Invalid pattern: {:?}", tok.kind),
                span: tok.span,
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use fusty_lexer::Lexer;

    #[test]
    fn test_parse_boss_fight() {
        let code = r#"
            enum BossState {
                Idle,
                Attack,
            }

            struct Boss {
                hp: i32,
                pos: vec2,
            }

            nya update_boss(mut self, dt: f32) {
                let move_vec = vec2(10.0, 20.0);
                self.pos += move_vec * dt;

                wait 0.25;

                meow {
                    let raw = 1337;
                }
            }
        "#;

        let mut lexer = Lexer::new(code);
        let tokens = lexer.tokenize().expect("Lex failed");
        let mut parser = Parser::new(tokens);
        let program = parser.parse_program().expect("Parse failed");

        assert_eq!(program.items.len(), 3); // Enum, Struct, Fn
    }

    #[test]
    fn test_parse_variable_declarations() {
        let code = "let x = 10; let mut y = 20.5; let v = vec2(1.0, 2.0);";
        let mut lexer = Lexer::new(code);
        let tokens = lexer.tokenize().expect("Lex failed");
        let mut parser = Parser::new(tokens);
        let program = parser.parse_program().expect("Parse failed");
        assert_eq!(program.items.len(), 3);
    }

    #[test]
    fn test_parse_baka_and_tsundere_error() {
        let code = r#"
            nya check_hp(hp: i64) {
                if hp <= 0 {
                    baka!("no more headpats left");
                }
                baka!();
            }
        "#;
        let tokens = Lexer::new(code).tokenize().expect("Lex failed");
        let program = Parser::new(tokens).parse_program().expect("Parse failed");
        assert_eq!(program.items.len(), 1);

        let bad_tokens = Lexer::new("nya broken( {}").tokenize().expect("Lex failed");
        let err = Parser::new(bad_tokens).parse_program().unwrap_err();
        assert!(err.message.contains("B-baka!"));
    }
}
