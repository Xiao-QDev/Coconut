#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Package,
    Import,
    Fn,
    Var,
    Const,
    If,
    Else,
    For,
    Return,
    Break,
    Continue,
    Extern,

    Identifier(String),
    IntLiteral(i64),
    FloatLiteral(f64),
    StringLiteral(String),

    TypeInt,
    TypeString,
    TypeVoid,

    Plus,
    Minus,
    Star,
    Slash,
    Equal,
    ColonEqual,
    Bang,
    EqualEqual,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    Ampersand,

    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    Comma,
    Semicolon,
    Dot,

    EOF,
}

#[derive(Debug, Clone)]
pub struct TokenWithSpan {
    pub token: Token,
    pub line: usize,
}
