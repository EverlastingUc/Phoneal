#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Create,
    Mutable,
    Set,
    Print,
    Function,
    Return,
    Returns,
    If,
    Else,
    True,
    False,
    And,
    Or,
    Not,
    Colon,
    Equal,
    EqualEqual,
    BangEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Identifier(String),
    Integer(i64),
    String(String),
    LeftParen,
    RightParen,
    LeftBrace,
    RightBrace,
    Comma,
    Semicolon,
    Eof,
}
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub line: usize,
    pub column: usize,
}
impl Token {
    pub fn new(kind: TokenKind, line: usize, column: usize) -> Self {
        Self { kind, line, column }
    }
}
