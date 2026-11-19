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
        if self.take(&TokenKind::Fn) {
            return self.parse_function();
        }
        if self.take(&TokenKind::LeftBrace) {
            return self.parse_block();
        }
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
                    ))
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
        if self.take(&TokenKind::Return) {
            let token = &self.tokens[self.position - 1];
            let location = SourceLocation {
                line: token.line,
                column: token.column,
            };
            let value = self.parse_expression()?;
            self.require(&TokenKind::Semicolon, "Expected ';' after return value")?;
            return Ok(Statement::Return { value, location });
        }
        let token = self.peek();
        Err(Diagnostic::parsing(
            "Expected let, print, fn, return, or block",
            token.line,
            token.column,
        ))
    }

    fn parse_function(&mut self) -> Result<Statement, Diagnostic> {
        let name_token = self.advance();
        let name_location = SourceLocation {
            line: name_token.line,
            column: name_token.column,
        };
        let name = match name_token.kind {
            TokenKind::Identifier(name) => name,
            _ => {
                return Err(Diagnostic::parsing(
                    "Expected function name after fn",
                    name_token.line,
                    name_token.column,
                ))
            }
        };
        self.require(&TokenKind::LeftParen, "Expected '(' after function name")?;
        let mut parameters = Vec::new();
        if !self.take(&TokenKind::RightParen) {
            loop {
                let token = self.advance();
                let parameter = match token.kind {
                    TokenKind::Identifier(name) => name,
                    _ => {
                        return Err(Diagnostic::parsing(
                            "Expected parameter name",
                            token.line,
                            token.column,
                        ))
                    }
                };
                parameters.push((
                    parameter,
                    SourceLocation {
                        line: token.line,
                        column: token.column,
                    },
                ));
                if self.take(&TokenKind::RightParen) {
                    break;
                }
                self.require(&TokenKind::Comma, "Expected ',' or ')' after parameter")?;
            }
        }
        self.require(&TokenKind::LeftBrace, "Expected '{' before function body")?;
        let body = self.parse_block_contents()?;
        Ok(Statement::Function {
            name,
            name_location,
            parameters,
            body,
        })
    }

    fn parse_block(&mut self) -> Result<Statement, Diagnostic> {
        let opening = &self.tokens[self.position - 1];
        let location = SourceLocation {
            line: opening.line,
            column: opening.column,
        };
        let statements = self.parse_block_contents()?;
        Ok(Statement::Block {
            statements,
            location,
        })
    }

    fn parse_block_contents(&mut self) -> Result<Vec<Statement>, Diagnostic> {
        let mut statements = Vec::new();
        while !matches!(self.peek().kind, TokenKind::RightBrace | TokenKind::Eof) {
            statements.push(self.parse_statement()?);
        }
        self.require(&TokenKind::RightBrace, "Expected '}' to close block")?;
        Ok(statements)
    }

    fn parse_expression(&mut self) -> Result<Expression, Diagnostic> {
        self.parse_addition()
    }
    fn parse_addition(&mut self) -> Result<Expression, Diagnostic> {
        let mut expression = self.parse_multiplication()?;
        loop {
            let token = match self.peek().kind.clone() {
                TokenKind::Plus | TokenKind::Minus => Some(self.advance()),
                _ => None,
            };
            let Some(token) = token else { break };
            let operator_location = SourceLocation {
                line: token.line,
                column: token.column,
            };
            let right = self.parse_multiplication()?;
            expression = Expression::Binary {
                left: Box::new(expression),
                operator: token.kind,
                operator_location,
                right: Box::new(right),
            };
        }
        Ok(expression)
    }
    fn parse_multiplication(&mut self) -> Result<Expression, Diagnostic> {
        let mut expression = self.parse_primary()?;
        loop {
            let token = match self.peek().kind {
                TokenKind::Star | TokenKind::Slash => Some(self.advance()),
                _ => None,
            };
            let Some(token) = token else { break };
            let operator_location = SourceLocation {
                line: token.line,
                column: token.column,
            };
            let right = self.parse_primary()?;
            expression = Expression::Binary {
                left: Box::new(expression),
                operator: token.kind,
                operator_location,
                right: Box::new(right),
            };
        }
        Ok(expression)
    }
    fn parse_primary(&mut self) -> Result<Expression, Diagnostic> {
        let token = self.advance();
        match token.kind {
            TokenKind::Integer(value) => Ok(Expression::Integer(value)),
            TokenKind::String(value) => Ok(Expression::String(value)),
            TokenKind::Identifier(name) => {
                let location = SourceLocation {
                    line: token.line,
                    column: token.column,
                };
                if self.take(&TokenKind::LeftParen) {
                    let mut arguments = Vec::new();
                    if !self.take(&TokenKind::RightParen) {
                        loop {
                            arguments.push(self.parse_expression()?);
                            if self.take(&TokenKind::RightParen) {
                                break;
                            }
                            self.require(&TokenKind::Comma, "Expected ',' or ')' after argument")?;
                        }
                    }
                    Ok(Expression::Call {
                        name,
                        location,
                        arguments,
                    })
                } else {
                    Ok(Expression::Variable { name, location })
                }
            }
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
    use crate::lexer::Lexer;

    #[test]
    fn parses_function_declarations_and_calls() {
        let source = "fn add(a, b) { return a + b; } let result = add(20, 22);";
        let statements = Parser::new(Lexer::new(source).scan().unwrap())
            .parse()
            .unwrap();
        assert!(matches!(statements[0], Statement::Function { .. }));
        let Statement::Let { value, .. } = &statements[1] else {
            panic!("expected let");
        };
        assert!(matches!(value, Expression::Call { arguments, .. } if arguments.len() == 2));
    }

    #[test]
    fn rejects_duplicate_parameter_syntax_later_in_semantic_analysis() {
        let source = "fn f(a, a) { return a; }";
        assert!(Parser::new(Lexer::new(source).scan().unwrap())
            .parse()
            .is_ok());
    }

    #[test]
    fn parses_nested_blocks() {
        let source = "{ let outer = 1; { let inner = 2; print(inner); } print(outer); }";
        let statements = Parser::new(Lexer::new(source).scan().unwrap())
            .parse()
            .unwrap();
        assert!(matches!(statements[0], Statement::Block { .. }));
    }

    #[test]
    fn rejects_unclosed_block_at_end_of_file() {
        let error = Parser::new(Lexer::new("{ let x = 1;").scan().unwrap())
            .parse()
            .unwrap_err();
        assert!(error.message.contains("Expected '}'"));
    }
}
