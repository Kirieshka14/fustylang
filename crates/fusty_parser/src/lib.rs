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
        self.tokens.get(self.cursor).unwrap_or_else(|| {
            self.tokens.last().expect("Tokens should not be empty")
        })
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
                message: format!("Expected {}, found {:?}", msg, tok.kind),
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
        if self.check(&TokenKind::Meow) && self.peek_next_kind() == Some(&TokenKind::Fn) {
            self.advance(); // consume `meow`
            let mut func = self.parse_fn()?;
            func.is_meow = true;
            return Ok(Item::Fn(func));
        }

        match self.peek_kind() {
            TokenKind::Fn => Ok(Item::Fn(self.parse_fn()?)),
            TokenKind::Struct => Ok(Item::Struct(self.parse_struct()?)),
            TokenKind::Enum => Ok(Item::Enum(self.parse_enum()?)),
            TokenKind::Impl => Ok(Item::Impl(self.parse_impl()?)),
            _ => Ok(Item::Stmt(self.parse_stmt()?)),
        }
    }


}
