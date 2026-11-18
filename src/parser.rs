use crate::{
    ast::{Expression, Statement},
    diagnostic::{Diagnostic, SourceLocation},
    token::{Token, TokenKind},
};

pub struct Parser {
    tokens: Vec<Token>,
    position: usize,
}

impl Parser {
    pub fn new(mut tokens: Vec<Token>) -> Self {
        if tokens
            .last()
            .is_none_or(|token| token.kind != TokenKind::Eof)
        {
            let (line, column) = tokens
                .last()
                .map(|token| (token.line, token.column + 1))
                .unwrap_or((1, 1));
            tokens.push(Token::new(TokenKind::Eof, line, column));
        }

        Self {
            tokens,
            position: 0,
        }
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.position]
    }

    fn advance(&mut self) -> Token {
        let token = self.peek().clone();

        if token.kind != TokenKind::Eof {
            self.position += 1;
        }

        token
    }

    fn take(&mut self, expected: &TokenKind) -> bool {
        if &self.peek().kind == expected {
            self.advance();
            true
        } else {
            false
        }
    }

    fn require(&mut self, expected: &TokenKind, message: &str) -> Result<(), Diagnostic> {
        if self.take(expected) {
            Ok(())
        } else {
            let token = self.peek();
            Err(Diagnostic::parsing(message, token.line, token.column))
        }
    }

    pub fn parse(&mut self) -> Result<Vec<Statement>, Diagnostic> {
        let mut statements = Vec::new();

        while self.peek().kind != TokenKind::Eof {
            statements.push(self.parse_statement()?);
        }

        Ok(statements)
    }

    fn parse_statement(&mut self) -> Result<Statement, Diagnostic> {
        if self.take(&TokenKind::Let) {
            let token = self.advance();
            let name_location = SourceLocation {
                line: token.line,
                column: token.column,
            };
            let name = match token.kind {
                TokenKind::Identifier(name) => name,
                _ => {
                    return Err(Diagnostic::parsing(
                        "Expected variable name after let",
                        token.line,
                        token.column,
                    ));
                }
            };

            self.require(&TokenKind::Equal, "Expected '='")?;
            let value = self.parse_expression()?;
            self.require(&TokenKind::Semicolon, "Expected ';'")?;

            return Ok(Statement::Let {
                name,
                name_location,
                value,
            });
        }

        if self.take(&TokenKind::Print) {
            self.require(&TokenKind::LeftParen, "Expected '('")?;
            let expression = self.parse_expression()?;
            self.require(&TokenKind::RightParen, "Expected ')'")?;
            self.require(&TokenKind::Semicolon, "Expected ';'")?;

            return Ok(Statement::Print(expression));
        }

        let token = self.peek();
        Err(Diagnostic::parsing(
            "Expected let or print",
            token.line,
            token.column,
        ))
    }

    fn parse_expression(&mut self) -> Result<Expression, Diagnostic> {
        self.parse_addition()
    }

    fn parse_addition(&mut self) -> Result<Expression, Diagnostic> {
        let mut expression = self.parse_multiplication()?;

        loop {
            let operator = if self.take(&TokenKind::Plus) {
                Some(TokenKind::Plus)
            } else if self.take(&TokenKind::Minus) {
                Some(TokenKind::Minus)
            } else {
                None
            };

            match operator {
                Some(operator) => {
                    let right = self.parse_multiplication()?;
                    expression = Expression::Binary {
                        left: Box::new(expression),
                        operator,
                        right: Box::new(right),
                    };
                }
                None => break,
            }
        }

        Ok(expression)
    }

    fn parse_multiplication(&mut self) -> Result<Expression, Diagnostic> {
        let mut expression = self.parse_primary()?;

        loop {
            let operator = if self.take(&TokenKind::Star) {
                Some(TokenKind::Star)
            } else if self.take(&TokenKind::Slash) {
                Some(TokenKind::Slash)
            } else {
                None
            };

            match operator {
                Some(operator) => {
                    let right = self.parse_primary()?;
                    expression = Expression::Binary {
                        left: Box::new(expression),
                        operator,
                        right: Box::new(right),
                    };
                }
                None => break,
            }
        }

        Ok(expression)
    }

    fn parse_primary(&mut self) -> Result<Expression, Diagnostic> {
        let token = self.advance();

        match token.kind {
            TokenKind::Integer(value) => Ok(Expression::Integer(value)),
            TokenKind::String(value) => Ok(Expression::String(value)),
            TokenKind::Identifier(name) => Ok(Expression::Variable {
                name,
                location: SourceLocation {
                    line: token.line,
                    column: token.column,
                },
            }),
            TokenKind::LeftParen => {
                let expression = self.parse_expression()?;
                self.require(&TokenKind::RightParen, "Expected ')'")?;
                Ok(Expression::Group(Box::new(expression)))
            }
            _ => Err(Diagnostic::parsing(
                "Expected expression",
                token.line,
                token.column,
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{interpreter::evaluate, lexer::Lexer};
    use std::collections::HashMap;

    #[test]
    fn multiplication_has_higher_precedence_than_addition() {
        let tokens = Lexer::new("let result = 2 + 3 * 4;").scan().unwrap();
        let statements = Parser::new(tokens).parse().unwrap();

        let Statement::Let { value, .. } = &statements[0] else {
            panic!("expected let statement");
        };

        assert_eq!(evaluate(value, &HashMap::new()).unwrap().to_string(), "14");
    }

    #[test]
    fn reports_missing_semicolon_with_location() {
        let error = Parser::new(Lexer::new("let answer = 42").scan().unwrap())
            .parse()
            .unwrap_err();

        assert!(error.to_string().contains("Expected ';'"));
        assert!(error.to_string().contains("at 1:16"));
    }

    #[test]
    fn empty_token_stream_is_a_safe_empty_program() {
        assert!(Parser::new(Vec::new()).parse().unwrap().is_empty());
    }

    #[test]
    fn token_stream_without_eof_is_normalized() {
        let error = Parser::new(vec![Token::new(TokenKind::Let, 1, 1)])
            .parse()
            .unwrap_err();

        assert!(error.to_string().contains("Expected variable name"));
        assert!(error.to_string().contains("at 1:2"));
    }

    #[test]
    fn missing_variable_name_reports_offending_token_location() {
        let error = Parser::new(Lexer::new("let = 10;").scan().unwrap())
            .parse()
            .unwrap_err();

        assert!(error.to_string().contains("at 1:5"));
    }

    #[test]
    fn preserves_identifier_source_locations() {
        let statements = Parser::new(Lexer::new("let result = value;").scan().unwrap())
            .parse()
            .unwrap();
        let Statement::Let {
            name_location,
            value,
            ..
        } = &statements[0]
        else {
            panic!("expected let statement");
        };

        assert_eq!(name_location.line, 1);
        assert_eq!(name_location.column, 5);
        let Expression::Variable { location, .. } = value else {
            panic!("expected variable expression");
        };
        assert_eq!(location.line, 1);
        assert_eq!(location.column, 14);
    }
}
