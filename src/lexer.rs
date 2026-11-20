use crate::{
    diagnostic::Diagnostic,
    token::{Token, TokenKind},
};

/// Maximum UTF-8 source size accepted by the prototype lexer.
pub const MAX_SOURCE_BYTES: usize = 1_048_576;
/// Maximum number of non-EOF tokens accepted from one source file.
pub const MAX_TOKENS: usize = 100_000;
pub struct Lexer {
    characters: Vec<char>,
    position: usize,
    line: usize,
    column: usize,
    source_too_large: bool,
}
impl Lexer {
    pub fn new(source: &str) -> Self {
        Self {
            characters: source.chars().collect(),
            position: 0,
            line: 1,
            column: 1,
            source_too_large: source.len() > MAX_SOURCE_BYTES,
        }
    }
    fn peek(&self) -> Option<char> {
        self.characters.get(self.position).copied()
    }
    fn peek_next(&self) -> Option<char> {
        self.characters.get(self.position + 1).copied()
    }
    fn advance(&mut self) -> char {
        let c = self.characters[self.position];
        self.position += 1;
        if c == '\n' {
            self.line += 1;
            self.column = 1
        } else {
            self.column += 1
        }
        c
    }
    pub fn scan(mut self) -> Result<Vec<Token>, Diagnostic> {
        if self.source_too_large {
            return Err(Diagnostic::lexical(
                format!("Source exceeds the {} byte limit", MAX_SOURCE_BYTES),
                1,
                1,
            ));
        }
        let mut tokens = Vec::new();
        while let Some(current) = self.peek() {
            let (line, column) = (self.line, self.column);
            if current.is_whitespace() {
                self.advance();
                continue;
            }
            if current == '#' {
                while self.peek().is_some_and(|c| c != '\n') {
                    self.advance();
                }
                continue;
            }
            if current == '/' && self.peek_next() == Some('*') {
                self.advance();
                self.advance();
                let mut closed = false;
                while self.peek().is_some() {
                    if self.peek() == Some('*') && self.peek_next() == Some('/') {
                        self.advance();
                        self.advance();
                        closed = true;
                        break;
                    }
                    self.advance();
                }
                if !closed {
                    return Err(Diagnostic::lexical(
                        "Unterminated block comment",
                        line,
                        column,
                    ));
                }
                continue;
            }
            let kind = match self.advance() {
                '=' => {
                    if self.peek() == Some('=') {
                        self.advance();
                        TokenKind::EqualEqual
                    } else {
                        TokenKind::Equal
                    }
                }
                '!' => {
                    if self.peek() == Some('=') {
                        self.advance();
                        TokenKind::BangEqual
                    } else {
                        TokenKind::Not
                    }
                }
                '<' => {
                    if self.peek() == Some('=') {
                        self.advance();
                        TokenKind::LessEqual
                    } else {
                        TokenKind::Less
                    }
                }
                '>' => {
                    if self.peek() == Some('=') {
                        self.advance();
                        TokenKind::GreaterEqual
                    } else {
                        TokenKind::Greater
                    }
                }
                '+' => TokenKind::Plus,
                '-' => TokenKind::Minus,
                '*' => TokenKind::Star,
                '/' => TokenKind::Slash,
                '%' => TokenKind::Percent,
                '(' => TokenKind::LeftParen,
                ')' => TokenKind::RightParen,
                '{' => TokenKind::LeftBrace,
                '}' => TokenKind::RightBrace,
                ',' => TokenKind::Comma,
                ':' => TokenKind::Colon,
                ';' => TokenKind::Semicolon,
                '"' => {
                    let mut value = String::new();
                    let mut closed = false;
                    while self.peek().is_some() {
                        match self.advance() {
                            '"' => {
                                closed = true;
                                break;
                            }
                            '\n' => {
                                return Err(Diagnostic::lexical(
                                    "Unterminated string",
                                    line,
                                    column,
                                ))
                            }
                            '\\' => {
                                let Some(next) = self.peek() else {
                                    return Err(Diagnostic::lexical(
                                        "Incomplete escape sequence",
                                        self.line,
                                        self.column,
                                    ));
                                };
                                let escaped = self.advance();
                                value.push(match escaped {
                                    'n' => '\n',
                                    'r' => '\r',
                                    't' => '\t',
                                    '"' => '"',
                                    '\\' => '\\',
                                    other => {
                                        return Err(Diagnostic::lexical(
                                            format!("Unknown escape sequence '\\{other}'"),
                                            self.line,
                                            self.column - 1,
                                        ))
                                    }
                                });
                                let _ = next;
                            }
                            other => value.push(other),
                        }
                    }
                    if !closed {
                        return Err(Diagnostic::lexical("Unterminated string", line, column));
                    }
                    TokenKind::String(value)
                }
                digit if digit.is_ascii_digit() => {
                    let mut n = digit.to_string();
                    while self.peek().is_some_and(|c| c.is_ascii_digit()) {
                        n.push(self.advance());
                    }
                    TokenKind::Integer(n.parse::<i64>().map_err(|_| {
                        Diagnostic::lexical("Integer literal is out of range", line, column)
                    })?)
                }
                first if first.is_ascii_alphabetic() || first == '_' => {
                    let mut word = first.to_string();
                    while self
                        .peek()
                        .is_some_and(|c| c.is_ascii_alphanumeric() || c == '_')
                    {
                        word.push(self.advance());
                    }
                    match word.as_str() {
                        "create" => TokenKind::Create,
                        "mutable" => TokenKind::Mutable,
                        "set" => TokenKind::Set,
                        "print" => TokenKind::Print,
                        "function" => TokenKind::Function,
                        "return" => TokenKind::Return,
                        "returns" => TokenKind::Returns,
                        "if" => TokenKind::If,
                        "else" => TokenKind::Else,
                        "while" => TokenKind::While,
                        "break" => TokenKind::Break,
                        "continue" => TokenKind::Continue,
                        "true" => TokenKind::True,
                        "false" => TokenKind::False,
                        "and" => TokenKind::And,
                        "or" => TokenKind::Or,
                        "not" => TokenKind::Not,
                        _ => TokenKind::Identifier(word),
                    }
                }
                other => {
                    return Err(Diagnostic::lexical(
                        format!("Unexpected character '{other}'"),
                        line,
                        column,
                    ))
                }
            };
            if tokens.len() >= MAX_TOKENS {
                return Err(Diagnostic::lexical(
                    format!("Source exceeds the {MAX_TOKENS} token limit"),
                    line,
                    column,
                ));
            }
            tokens.push(Token::new(kind, line, column));
        }
        tokens.push(Token::new(TokenKind::Eof, self.line, self.column));
        Ok(tokens)
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn scans_english_keywords_and_comparisons() {
        let t = Lexer::new("create x = true; function f() returns boolean { return x == false; }")
            .scan()
            .unwrap();
        assert_eq!(t[0].kind, TokenKind::Create);
        assert_eq!(t[5].kind, TokenKind::Function);
        assert!(t.iter().any(|x| x.kind == TokenKind::EqualEqual));
    }
    #[test]
    fn scans_loop_keywords() {
        let tokens = Lexer::new("while (true) { break; continue; }")
            .scan()
            .unwrap();
        assert_eq!(tokens[0].kind, TokenKind::While);
        assert_eq!(tokens[5].kind, TokenKind::Break);
        assert_eq!(tokens[7].kind, TokenKind::Continue);
    }
    #[test]
    fn scans_comments_and_escaped_strings() {
        let t = Lexer::new("# comment\ncreate s = \"a\\n b\"; /* ok */")
            .scan()
            .unwrap();
        assert_eq!(t[3].kind, TokenKind::String("a\n b".into()));
    }
    #[test]
    fn rejects_unterminated_block_comments() {
        assert!(Lexer::new("/* nope")
            .scan()
            .unwrap_err()
            .message
            .contains("Unterminated block comment"));
    }
    #[test]
    fn rejects_unterminated_strings_with_location() {
        let e = Lexer::new("print(\n  \"unfinished);").scan().unwrap_err();
        assert_eq!(e.location.unwrap().line, 2);
    }
    #[test]
    fn rejects_source_larger_than_configured_limit() {
        let source = " ".repeat(MAX_SOURCE_BYTES + 1);
        let error = Lexer::new(&source).scan().unwrap_err();
        assert!(error.message.contains("byte limit"));
    }
    #[test]
    fn rejects_token_floods() {
        let source = "print(1);".repeat(MAX_TOKENS / 4 + 1);
        let error = Lexer::new(&source).scan().unwrap_err();
        assert!(error.message.contains("token limit"));
    }
    #[test]
    fn rejects_integer_literals_outside_i64_range() {
        assert!(Lexer::new("create x = 9223372036854775808;")
            .scan()
            .is_err());
    }
}
