use crate::token::{Token, TokenKind};

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

    pub fn scan(mut self) -> Result<Vec<Token>, String> {
        let mut tokens = Vec::new();

        while let Some(character) = self.peek() {
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
                                return Err(format!(
                                    "Unterminated string at {line}:{column}"
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
                        format!("Integer out of range at {line}:{column}")
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
                    return Err(format!(
                        "Unexpected character '{unexpected}' at {line}:{column}"
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

    #[test]
    fn scans_keywords_identifiers_and_integers() {
        let tokens = Lexer::new("let count = 42;").scan().unwrap();

        assert_eq!(tokens[0].kind, TokenKind::Let);
        assert_eq!(
            tokens[1].kind,
            TokenKind::Identifier("count".to_owned())
        );
        assert_eq!(tokens[3].kind, TokenKind::Integer(42));
        assert_eq!(tokens[4].kind, TokenKind::Semicolon);
    }

    #[test]
    fn reports_unterminated_strings() {
        let error = Lexer::new("print(\"unfinished);").scan().unwrap_err();
        assert!(error.contains("Unterminated string at 1:7"));
    }
}
