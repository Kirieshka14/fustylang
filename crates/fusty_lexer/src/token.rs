#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Keywords
    Nya, // Function declaration: `nya foo() { ... }`
    Let,
    Mut,
    Struct,
    Enum,
    Impl,
    Match,
    If,
    Else,
    While,
    For,
    In,
    Return,
    Break,
    Continue,
    Meow, // The iconic safety bypass: `meow { ... }` or `meow nya ...`
    Uwu,  // Cuddle operator: `a uwu b` = midpoint of a and b
    Baka, // Tsundere panic replacement: `baka!("...")`
    Wait, // Game coroutine pause: `wait 0.2;`
    Yield,
    Spawn,
    True,
    False,
    SelfValue, // `self`
    SelfType,  // `Self`

    // Types & Builtins
    Vec2,
    Vec3,

    // Literals
    Ident(String),
    IntLit(i64),
    FloatLit(f64),
    StringLit(String),

    // Delimiters & Grouping
    LParen,   // (
    RParen,   // )
    LBrace,   // {
    RBrace,   // }
    LBracket, // [
    RBracket, // ]

    // Operators & Punctuation
    Comma,       // ,
    Colon,       // :
    DoubleColon, // ::
    Semicolon,   // ;
    Dot,         // .
    Arrow,       // ->
    FatArrow,    // =>
    Plus,        // +
    Minus,       // -
    Star,        // *
    Slash,       // /
    Percent,     // %
    Assign,      // =
    PlusAssign,  // +=
    MinusAssign, // -=
    StarAssign,  // *=
    SlashAssign, // /=
    Eq,          // ==
    NotEq,       // !=
    Lt,          // <
    LtEq,        // <=
    Gt,          // >
    GtEq,        // >=
    And,         // &&
    Or,          // ||
    Bang,        // !
    Underscore,  // _
    Question,    // ?

    // Special
    Eof,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
    pub line: usize,
    pub col: usize,
}

impl Span {
    pub fn new(start: usize, end: usize, line: usize, col: usize) -> Self {
        Self {
            start,
            end,
            line,
            col,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    pub fn new(kind: TokenKind, span: Span) -> Self {
        Self { kind, span }
    }
}
