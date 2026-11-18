use crate::{
    ast::{Expression, Statement},
    token::{Token, TokenKind},
};

pub struct Parser {
    tokens: Vec<Token>,
    position: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            position: 0,
        }
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.position]
    }

    fn advance(&mut self) -> Token {
        let token = self.tokens[self.position].clone();

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

    fn require(&mut self, expected: &TokenKind, message: &str) -> Result<(), String> {
        if self.take(expected) {
            Ok(())
        } else {
            let token = self.peek();
            Err(format!("{message} at {}:{}", token.line, token.column))
        }
    }

    pub fn parse(&mut self) -> Result<Vec<Statement>, String> {
        let mut statements = Vec::new();

        while self.peek().kind != TokenKind::Eof {
            statements.push(self.parse_statement()?);
        }

        Ok(statements)
    }

    fn parse_statement(&mut self) -> Result<Statement, String> {
        if self.take(&TokenKind::Let) {
            let name = match self.advance().kind {
                TokenKind::Identifier(name) => name,
                _ => return Err("Expected variable name after let".to_owned()),
            };

            self.require(&TokenKind::Equal, "Expected '='")?;
            let value = self.parse_expression()?;
            self.require(&TokenKind::Semicolon, "Expected ';'")?;

            return Ok(Statement::Let { name, value });
        }

        if self.take(&TokenKind::Print) {
            self.require(&TokenKind::LeftParen, "Expected '('")?;
            let expression = self.parse_expression()?;
            self.require(&TokenKind::RightParen, "Expected ')'")?;
            self.require(&TokenKind::Semicolon, "Expected ';'")?;

            return Ok(Statement::Print(expression));
        }

        let token = self.peek();
        Err(format!(
            "Expected let or print at {}:{}",
            token.line, token.column
        ))
    }

    fn parse_expression(&mut self) -> Result<Expression, String> {
        self.parse_addition()
    }

    fn parse_addition(&mut self) -> Result<Expression, String> {
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

    fn parse_multiplication(&mut self) -> Result<Expression, String> {
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

    fn parse_primary(&mut self) -> Result<Expression, String> {
        let token = self.advance();

        match token.kind {
            TokenKind::Integer(value) => Ok(Expression::Integer(value)),
            TokenKind::String(value) => Ok(Expression::String(value)),
            TokenKind::Identifier(name) => Ok(Expression::Variable(name)),
            TokenKind::LeftParen => {
                let expression = self.parse_expression()?;
                self.require(&TokenKind::RightParen, "Expected ')'")?;
                Ok(Expression::Group(Box::new(expression)))
            }
            _ => Err(format!(
                "Expected expression at {}:{}",
                token.line, token.column
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

        assert_eq!(
            evaluate(value, &HashMap::new()).unwrap().to_string(),
            "14"
        );
    }

    #[test]
    fn reports_missing_semicolon() {
        let tokens = Lexer::new("let answer = 42").scan().unwrap();
        let error = Parser::new(tokens).parse().unwrap_err();

        assert!(error.contains("Expected ';'"));
    }
}
