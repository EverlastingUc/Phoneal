use crate::{
    diagnostic::Diagnostic,
    token::{Token, TokenKind},
};

pub struct Lexer {
    characters: Vec<char>,
    position: usize,
    line: usize,
    column: usize,
}

impl Lexer {
    pub fn new(source: &str) -> Self {
        Self {
            characters: source.chars().collect(),
            position: 0,
            line: 1,
            column: 1,
        }
    }

    fn peek(&self) -> Option<char> {
        self.characters.get(self.position).copied()
    }

    fn advance(&mut self) -> char {
        let character = self.characters[self.position];
        self.position += 1;

        if character == '\n' {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }

        character
    }

    pub fn scan(mut self) -> Result<Vec<Token>, Diagnostic> {
        let mut tokens = Vec::new();

        while self.peek().is_some() {
            let (line, column) = (self.line, self.column);
            let kind = match self.advance() {
                ' ' | '\r' | '\t' | '\n' => continue,
                '=' => TokenKind::Equal,
                '+' => TokenKind::Plus,
                '-' => TokenKind::Minus,
                '*' => TokenKind::Star,
                '/' => TokenKind::Slash,
                '(' => TokenKind::LeftParen,
                ')' => TokenKind::RightParen,
                '{' => TokenKind::LeftBrace,
                '}' => TokenKind::RightBrace,
                ';' => TokenKind::Semicolon,
                '"' => {
                    let mut value = String::new();

                    loop {
                        match self.peek() {
                            Some('"') => {
                                self.advance();
                                break TokenKind::String(value);
                            }
                            Some('\n') | None => {
                                return Err(Diagnostic::lexical(
                                    "Unterminated string",
                                    line,
                                    column,
                                ));
                            }
                            Some(_) => value.push(self.advance()),
                        }
                    }
                }
                digit if digit.is_ascii_digit() => {
                    let mut number = digit.to_string();

                    while self.peek().is_some_and(|next| next.is_ascii_digit()) {
                        number.push(self.advance());
                    }

                    let value = number.parse::<i64>().map_err(|_| {
                        Diagnostic::lexical("Integer literal is out of range", line, column)
                    })?;

                    TokenKind::Integer(value)
                }
                first if first.is_ascii_alphabetic() || first == '_' => {
                    let mut identifier = first.to_string();

                    while self
                        .peek()
                        .is_some_and(|next| next.is_ascii_alphanumeric() || next == '_')
                    {
                        identifier.push(self.advance());
                    }

                    match identifier.as_str() {
                        "let" => TokenKind::Let,
                        "print" => TokenKind::Print,
                        _ => TokenKind::Identifier(identifier),
                    }
                }
                unexpected => {
                    return Err(Diagnostic::lexical(
                        format!("Unexpected character '{unexpected}'"),
                        line,
                        column,
                    ));
                }
            };

            tokens.push(Token::new(kind, line, column));
        }

        tokens.push(Token::new(TokenKind::Eof, self.line, self.column));
        Ok(tokens)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diagnostic::DiagnosticStage;

    #[test]
    fn scans_keywords_identifiers_and_integers() {
        let tokens = Lexer::new("let count = 42;").scan().unwrap();

        assert_eq!(tokens[0].kind, TokenKind::Let);
        assert_eq!(tokens[1].kind, TokenKind::Identifier("count".to_owned()));
        assert_eq!(tokens[3].kind, TokenKind::Integer(42));
        assert_eq!(tokens[4].kind, TokenKind::Semicolon);
    }

    #[test]
    fn scans_block_delimiters() {
        let tokens = Lexer::new("{ let x = 1; }").scan().unwrap();
        assert_eq!(tokens[0].kind, TokenKind::LeftBrace);
        assert_eq!(tokens[6].kind, TokenKind::RightBrace);
    }

    #[test]
    fn reports_unterminated_strings_with_location() {
        let error = Lexer::new("print(\n  \"unfinished);").scan().unwrap_err();

        assert_eq!(error.stage, DiagnosticStage::Lexing);
        assert_eq!(error.location.as_ref().unwrap().line, 2);
        assert!(error.to_string().contains("at 2:3"));
    }

    #[test]
    fn rejects_integer_literals_outside_i64_range() {
        let error = Lexer::new("let value = 9223372036854775808;")
            .scan()
            .unwrap_err();

        assert!(error.message.contains("out of range"));
        assert_eq!(error.location.unwrap().column, 13);
    }
}
