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


}
