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
        self.chars
            .get(self.cursor)
            .map(|&(i, _)| i)
            .unwrap_or(self.input.len())
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
                        return Err((
                            "B-baka! Did you mean '&&'? Single '&' isn't supported here... >///<"
                                .into(),
                            span,
                        ));
                    }
                }

                '|' => {
                    if self.peek() == Some('|') {
                        self.advance();
                        TokenKind::Or
                    } else {
                        let span = Span::new(start_pos, self.current_pos(), start_line, start_col);
                        return Err((
                            "Hmph! Expected '||', single '|' isn't supported, dummy! >.<".into(),
                            span,
                        ));
                    }
                }

                '"' => self.lex_string(start_pos, start_line, start_col)?,

                '0'..='9' => self.lex_number(c, start_pos, start_line, start_col)?,

                'a'..='z' | 'A'..='Z' | '_' => self.lex_ident(c, start_pos, start_line, start_col),

                other => {
                    let span = Span::new(start_pos, self.current_pos(), start_line, start_col);
                    return Err((
                        format!(
                            "B-baka! What is '{other}' doing in my code?! Not like I care or anything... >///<"
                        ),
                        span,
                    ));
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
                                let span =
                                    Span::new(start_pos, self.current_pos(), start_line, start_col);
                                return Err((
                                    "Hmph! You forgot to close your multi-line comment '*/', b-baka! >.<"
                                        .into(),
                                    span,
                                ));
                            }
                        }
                    }
                }
                _ => break,
            }
        }
        Ok(())
    }

    fn lex_string(
        &mut self,
        start_pos: usize,
        line: usize,
        col: usize,
    ) -> Result<TokenKind, (String, Span)> {
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
                        return Err((
                            "B-baka! Unterminated escape sequence in your string literal! >///<"
                                .into(),
                            span,
                        ));
                    }
                },
                Some(c) => s.push(c),
                None => {
                    let span = Span::new(start_pos, self.current_pos(), line, col);
                    return Err((
                        "Hmph! Close your string literal with '\"', dummy! I'm not guessing where it ends! >.<"
                            .into(),
                        span,
                    ));
                }
            }
        }
        Ok(TokenKind::StringLit(s))
    }

    fn lex_number(
        &mut self,
        first_char: char,
        start_pos: usize,
        line: usize,
        col: usize,
    ) -> Result<TokenKind, (String, Span)> {
        let mut num_str = String::new();
        num_str.push(first_char);
        let mut is_float = false;

        while let Some(c) = self.peek() {
            if c.is_ascii_digit() || c == '_' {
                if c != '_' {
                    num_str.push(c);
                }
                self.advance();
            } else if !is_float
                && c == '.'
                && self
                    .peek_next()
                    .map(|n| n.is_ascii_digit())
                    .unwrap_or(false)
            {
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
                    Err((
                        format!("B-baka! That's not a valid float ({e})! Check your digits! >///<"),
                        span,
                    ))
                }
            }
        } else {
            match num_str.parse::<i64>() {
                Ok(val) => Ok(TokenKind::IntLit(val)),
                Err(e) => {
                    let span = Span::new(start_pos, self.current_pos(), line, col);
                    Err((
                        format!("Hmph! That integer is way too weird ({e})! Fix it, b-baka! >.<"),
                        span,
                    ))
                }
            }
        }
    }

    fn lex_ident(
        &mut self,
        first_char: char,
        _start_pos: usize,
        _line: usize,
        _col: usize,
    ) -> TokenKind {
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
            "nya" => TokenKind::Nya,
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
            "meow" => TokenKind::Meow, // Unsafe replacement!
            "baka" => TokenKind::Baka, // Panic replacement!
            "wait" => TokenKind::Wait, // Coroutine sleep!
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
            nya update(mut self, dt: f32) {
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
        assert!(kinds.contains(&TokenKind::Nya));
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

    #[test]
    fn test_lexer_strings_and_nested_comments() {
        let code = r#"
            /* outer /* inner */ still comment */
            let msg = "hello fusty";
        "#;
        let mut lexer = Lexer::new(code);
        let tokens = lexer.tokenize().expect("Tokenization failed");
        let kinds: Vec<_> = tokens.into_iter().map(|t| t.kind).collect();
        assert!(kinds.contains(&TokenKind::StringLit("hello fusty".to_string())));
    }

    #[test]
    fn test_lexer_keywords_and_identifiers() {
        let mut lexer = Lexer::new("let mut player_hp = 100; yield; return vec3(1.0, 2.0, 3.0);");
        let tokens = lexer.tokenize().expect("Tokenization failed");
        let kinds: Vec<_> = tokens.into_iter().map(|t| t.kind).collect();
        assert!(kinds.contains(&TokenKind::Let));
        assert!(kinds.contains(&TokenKind::Mut));
        assert!(kinds.contains(&TokenKind::Yield));
        assert!(kinds.contains(&TokenKind::Return));
        assert!(kinds.contains(&TokenKind::Vec3));
        assert!(kinds.contains(&TokenKind::Ident("player_hp".to_string())));
    }

    #[test]
    fn test_lexer_baka_and_tsundere_errors() {
        let mut lexer = Lexer::new(r#"nya oops() { baka!("stop it!"); }"#);
        let tokens = lexer.tokenize().expect("Tokenization failed");
        let kinds: Vec<_> = tokens.into_iter().map(|t| t.kind).collect();
        assert!(kinds.contains(&TokenKind::Nya));
        assert!(kinds.contains(&TokenKind::Baka));
        assert!(kinds.contains(&TokenKind::Bang));

        let (err, _) = Lexer::new("let x = @;").tokenize().unwrap_err();
        assert!(err.contains("B-baka!"));
    }
}
