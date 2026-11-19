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


}
