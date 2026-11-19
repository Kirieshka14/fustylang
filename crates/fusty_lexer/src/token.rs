#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Keywords
    Fn,
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
    Meow,   // The iconic safety bypass: `meow { ... }` or `meow fn ...`
    Wait,   // Game coroutine pause: `wait 0.2;`
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
    Comma,      // ,
    Colon,      // :
    DoubleColon,// ::
    Semicolon,  // ;
    Dot,        // .
    Arrow,      // ->
    FatArrow,   // =>
    Plus,       // +
    Minus,      // -
    Star,       // *
    Slash,      // /
    Percent,    // %
    Assign,     // =
    PlusAssign, // +=
    MinusAssign,// -=
    StarAssign, // *=
    SlashAssign,// /=
    Eq,         // ==
    NotEq,      // !=
    Lt,         // <
    LtEq,       // <=
    Gt,         // >
    GtEq,       // >=
    And,        // &&
    Or,         // ||
    Bang,       // !
    Underscore, // _
    Question,   // ?

    // Special
    Eof,
}

