pub mod token;

use token::{Span, Token, TokenKind};

pub struct Lexer<'a> {
    input: &'a str,
    chars: Vec<(usize, char)>,
    cursor: usize,
    line: usize,
    line_start_cursor: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        let chars: Vec<(usize, char)> = input.char_indices().collect();
        Self {
            input,
            chars,
            cursor: 0,
            line: 1,
            line_start_cursor: 0,
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.cursor).map(|&(_, c)| c)
    }

    fn peek_next(&self) -> Option<char> {
        self.chars.get(self.cursor + 1).map(|&(_, c)| c)
    }

    fn advance(&mut self) -> Option<char> {
        if let Some(&(_, c)) = self.chars.get(self.cursor) {
            self.cursor += 1;
            if c == '\n' {
                self.line += 1;
                self.line_start_cursor = self.cursor;
            }
            Some(c)
        } else {
            None
        }
    }

    fn current_pos(&self) -> usize {
        self.chars.get(self.cursor).map(|&(i, _)| i).unwrap_or(self.input.len())
    }

    fn current_col(&self) -> usize {
        self.cursor.saturating_sub(self.line_start_cursor) + 1
    }

    pub fn tokenize(&mut self) -> Result<Vec<Token>, (String, Span)> {
        let mut tokens = Vec::new();

        loop {
            self.skip_whitespace_and_comments()?;

            let start_pos = self.current_pos();
            let start_col = self.current_col();
            let start_line = self.line;

            let c = match self.advance() {
                Some(c) => c,
                None => {
                    tokens.push(Token::new(
                        TokenKind::Eof,
                        Span::new(start_pos, start_pos, start_line, start_col),
                    ));
                    break;
                }
            };

            let kind = match c {
                '(' => TokenKind::LParen,
                ')' => TokenKind::RParen,
                '{' => TokenKind::LBrace,
                '}' => TokenKind::RBrace,
                '[' => TokenKind::LBracket,
                ']' => TokenKind::RBracket,
                ',' => TokenKind::Comma,
                ';' => TokenKind::Semicolon,
                '?' => TokenKind::Question,
                '.' => TokenKind::Dot,

                ':' => {
                    if self.peek() == Some(':') {
                        self.advance();
                        TokenKind::DoubleColon
                    } else {
                        TokenKind::Colon
                    }
                }

                '+' => {
                    if self.peek() == Some('=') {
                        self.advance();
                        TokenKind::PlusAssign
                    } else {
                        TokenKind::Plus
                    }
                }

                '-' => {
                    if self.peek() == Some('>') {
                        self.advance();
                        TokenKind::Arrow
                    } else if self.peek() == Some('=') {
                        self.advance();
                        TokenKind::MinusAssign
                    } else {
                        TokenKind::Minus
                    }
                }

                '*' => {
                    if self.peek() == Some('=') {
                        self.advance();
                        TokenKind::StarAssign
                    } else {
                        TokenKind::Star
                    }
                }

                '/' => {
                    if self.peek() == Some('=') {
                        self.advance();
                        TokenKind::SlashAssign
                    } else {
                        TokenKind::Slash
                    }
                }

                '%' => TokenKind::Percent,

                '=' => {
                    if self.peek() == Some('=') {
                        self.advance();
                        TokenKind::Eq
                    } else if self.peek() == Some('>') {
                        self.advance();
                        TokenKind::FatArrow
                    } else {
                        TokenKind::Assign
                    }
                }

                '!' => {
                    if self.peek() == Some('=') {
                        self.advance();
                        TokenKind::NotEq
                    } else {
                        TokenKind::Bang
                    }
                }

                '<' => {
                    if self.peek() == Some('=') {
                        self.advance();
                        TokenKind::LtEq
                    } else {
                        TokenKind::Lt
                    }
                }

                '>' => {
                    if self.peek() == Some('=') {
                        self.advance();
                        TokenKind::GtEq
                    } else {
                        TokenKind::Gt
                    }
                }

                '&' => {
                    if self.peek() == Some('&') {
                        self.advance();
                        TokenKind::And
                    } else {
                        let span = Span::new(start_pos, self.current_pos(), start_line, start_col);
                        return Err(("Expected '&&', single '&' is not supported".into(), span));
                    }
                }

                '|' => {
                    if self.peek() == Some('|') {
                        self.advance();
                        TokenKind::Or
                    } else {
                        let span = Span::new(start_pos, self.current_pos(), start_line, start_col);
                        return Err(("Expected '||', single '|' is not supported".into(), span));
                    }
                }

                '"' => self.lex_string(start_pos, start_line, start_col)?,

                '0'..='9' => self.lex_number(c, start_pos, start_line, start_col)?,

                'a'..='z' | 'A'..='Z' | '_' => self.lex_ident(c, start_pos, start_line, start_col),

                other => {
                    let span = Span::new(start_pos, self.current_pos(), start_line, start_col);
                    return Err((format!("Unexpected character: '{other}'"), span));
                }
            };

            let end_pos = self.current_pos();
            tokens.push(Token::new(
                kind,
                Span::new(start_pos, end_pos, start_line, start_col),
            ));
        }

        Ok(tokens)
    }

    fn skip_whitespace_and_comments(&mut self) -> Result<(), (String, Span)> {
        loop {
            match self.peek() {
                Some(' ' | '\t' | '\r' | '\n') => {
                    self.advance();
                }
                Some('/') if self.peek_next() == Some('/') => {
                    // Single-line comment
                    while let Some(c) = self.peek() {
                        if c == '\n' {
                            break;
                        }
                        self.advance();
                    }
                }
                Some('/') if self.peek_next() == Some('*') => {
                    // Multi-line comment (supports nesting, errors if unterminated)
                    let start_pos = self.current_pos();
                    let start_line = self.line;
                    let start_col = self.current_col();
                    self.advance(); // consume '/'
                    self.advance(); // consume '*'
                    let mut depth = 1usize;
                    while depth > 0 {
                        match self.peek() {
                            Some('/') if self.peek_next() == Some('*') => {
                                self.advance();
                                self.advance();
                                depth += 1;
                            }
                            Some('*') if self.peek_next() == Some('/') => {
                                self.advance();
                                self.advance();
                                depth -= 1;
                            }
                            Some(_) => {
                                self.advance();
                            }
                            None => {
                                let span = Span::new(start_pos, self.current_pos(), start_line, start_col);
                                return Err(("Unterminated multi-line comment".into(), span));
                            }
                        }
                    }
                }
                _ => break,
            }
        }
        Ok(())
    }

    fn lex_string(&mut self, start_pos: usize, line: usize, col: usize) -> Result<TokenKind, (String, Span)> {
        let mut s = String::new();
        loop {
            match self.advance() {
                Some('"') => break,
                Some('\\') => match self.advance() {
                    Some('n') => s.push('\n'),
                    Some('r') => s.push('\r'),
                    Some('t') => s.push('\t'),
                    Some('\\') => s.push('\\'),
                    Some('"') => s.push('"'),
                    Some(c) => s.push(c),
                    None => {
                        let span = Span::new(start_pos, self.current_pos(), line, col);
                        return Err(("Unterminated escape in string literal".into(), span));
                    }
                },
                Some(c) => s.push(c),
                None => {
                    let span = Span::new(start_pos, self.current_pos(), line, col);
                    return Err(("Unterminated string literal".into(), span));
                }
            }
        }
        Ok(TokenKind::StringLit(s))
    }

    fn lex_number(&mut self, first_char: char, start_pos: usize, line: usize, col: usize) -> Result<TokenKind, (String, Span)> {
        let mut num_str = String::new();
        num_str.push(first_char);
        let mut is_float = false;

        while let Some(c) = self.peek() {
            if c.is_ascii_digit() || c == '_' {
                if c != '_' {
                    num_str.push(c);
                }
                self.advance();
            } else if !is_float && c == '.' && self.peek_next().map(|n| n.is_ascii_digit()).unwrap_or(false) {
                is_float = true;
                num_str.push(c);
                self.advance(); // consume '.'
            } else {
                break;
            }
        }

        if is_float {
            match num_str.parse::<f64>() {
                Ok(val) => Ok(TokenKind::FloatLit(val)),
                Err(e) => {
                    let span = Span::new(start_pos, self.current_pos(), line, col);
                    Err((format!("Invalid float: {e}"), span))
                }
            }
        } else {
            match num_str.parse::<i64>() {
                Ok(val) => Ok(TokenKind::IntLit(val)),
                Err(e) => {
                    let span = Span::new(start_pos, self.current_pos(), line, col);
                    Err((format!("Invalid integer: {e}"), span))
                }
            }
        }
    }

    fn lex_ident(&mut self, first_char: char, _start_pos: usize, _line: usize, _col: usize) -> TokenKind {
        let mut ident = String::new();
        ident.push(first_char);

        while let Some(c) = self.peek() {
            if c.is_alphanumeric() || c == '_' {
                ident.push(c);
                self.advance();
            } else {
                break;
            }
        }

        match ident.as_str() {
            "fn" => TokenKind::Fn,
            "let" => TokenKind::Let,
            "mut" => TokenKind::Mut,
            "struct" => TokenKind::Struct,
            "enum" => TokenKind::Enum,
            "impl" => TokenKind::Impl,
            "match" => TokenKind::Match,
            "if" => TokenKind::If,
            "else" => TokenKind::Else,
            "while" => TokenKind::While,
            "for" => TokenKind::For,
            "in" => TokenKind::In,
            "return" => TokenKind::Return,
            "break" => TokenKind::Break,
            "continue" => TokenKind::Continue,
            "meow" => TokenKind::Meow,     // Unsafe replacement!
            "wait" => TokenKind::Wait,     // Coroutine sleep!
            "yield" => TokenKind::Yield,
            "spawn" => TokenKind::Spawn,
            "true" => TokenKind::True,
            "false" => TokenKind::False,
            "self" => TokenKind::SelfValue,
            "Self" => TokenKind::SelfType,
            "vec2" => TokenKind::Vec2,
            "vec3" => TokenKind::Vec3,
            "_" => TokenKind::Underscore,
            _ => TokenKind::Ident(ident),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lexer_meow_and_vec2() {
        let code = r#"
            // Game combat script
            fn update(mut self, dt: f32) {
                let v = vec2(10.0, 20.0);
                wait 0.2;
                meow {
                    let raw_ptr = 42;
                }
            }
        "#;
        let mut lexer = Lexer::new(code);
        let tokens = lexer.tokenize().expect("Tokenization failed");
        
        let kinds: Vec<_> = tokens.into_iter().map(|t| t.kind).collect();
        assert!(kinds.contains(&TokenKind::Meow));
        assert!(kinds.contains(&TokenKind::Wait));
        assert!(kinds.contains(&TokenKind::Vec2));
    }

    #[test]
    fn test_lexer_numbers_and_operators() {
        let mut lexer = Lexer::new("let x = 42 + 3.14 * 2 - 1 / 5;");
        let tokens = lexer.tokenize().expect("Tokenization failed");
        let kinds: Vec<_> = tokens.into_iter().map(|t| t.kind).collect();
        assert!(kinds.contains(&TokenKind::IntLit(42)));
        assert!(kinds.contains(&TokenKind::FloatLit(3.14)));
        assert!(kinds.contains(&TokenKind::Plus));
        assert!(kinds.contains(&TokenKind::Star));
        assert!(kinds.contains(&TokenKind::Minus));
        assert!(kinds.contains(&TokenKind::Slash));
    }
}
