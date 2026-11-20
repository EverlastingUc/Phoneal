use crate::{
    ast::{Expression, Statement},
    diagnostic::{Diagnostic, SourceLocation},
    token::{Token, TokenKind},
    types::PhonealType,
};
/// Maximum recursive grammar nesting accepted before returning a diagnostic.
pub const MAX_NESTING_DEPTH: usize = 256;

pub struct Parser {
    tokens: Vec<Token>,
    position: usize,
    nesting_depth: usize,
}
impl Parser {
    pub fn new(mut tokens: Vec<Token>) -> Self {
        if tokens.last().is_none_or(|t| t.kind != TokenKind::Eof) {
            let (line, column) = tokens
                .last()
                .map(|t| (t.line, t.column + 1))
                .unwrap_or((1, 1));
            tokens.push(Token::new(TokenKind::Eof, line, column));
        }
        Self {
            tokens,
            position: 0,
            nesting_depth: 0,
        }
    }
    fn peek(&self) -> &Token {
        &self.tokens[self.position]
    }
    fn advance(&mut self) -> Token {
        let t = self.peek().clone();
        if t.kind != TokenKind::Eof {
            self.position += 1;
        }
        t
    }
    fn take(&mut self, k: &TokenKind) -> bool {
        if &self.peek().kind == k {
            self.advance();
            true
        } else {
            false
        }
    }
    fn require(&mut self, k: &TokenKind, msg: &str) -> Result<(), Diagnostic> {
        if self.take(k) {
            Ok(())
        } else {
            let t = self.peek();
            Err(Diagnostic::parsing(msg, t.line, t.column))
        }
    }
    fn loc(t: &Token) -> SourceLocation {
        SourceLocation {
            line: t.line,
            column: t.column,
        }
    }
    pub fn parse(&mut self) -> Result<Vec<Statement>, Diagnostic> {
        let mut out = Vec::new();
        while self.peek().kind != TokenKind::Eof {
            out.push(self.parse_statement()?);
        }
        Ok(out)
    }
    fn parse_statement(&mut self) -> Result<Statement, Diagnostic> {
        match self.peek().kind {
            TokenKind::Function => {
                self.advance();
                self.parse_function()
            }
            TokenKind::Create => {
                self.advance();
                self.parse_declaration()
            }
            TokenKind::Set => {
                self.advance();
                self.parse_assignment()
            }
            TokenKind::Print => {
                self.advance();
                self.parse_print()
            }
            TokenKind::Return => {
                let token = self.advance();
                self.parse_return(token)
            }
            TokenKind::If => {
                self.advance();
                self.parse_if()
            }
            TokenKind::While => {
                self.advance();
                self.parse_while()
            }
            TokenKind::Break | TokenKind::Continue => self.parse_loop_control(),
            TokenKind::LeftBrace => {
                self.advance();
                self.parse_block()
            }
            _ => {
                let token = self.peek();
                Err(Diagnostic::parsing(
                    "Expected create, set, print, function, if, while, break, continue, return, or a block",
                    token.line,
                    token.column,
                ))
            }
        }
    }
    fn parse_declaration(&mut self) -> Result<Statement, Diagnostic> {
        let mutable = self.take(&TokenKind::Mutable);
        let token = self.advance();
        let location = Self::loc(&token);
        let name = match token.kind {
            TokenKind::Identifier(name) => name,
            _ => {
                return Err(Diagnostic::parsing(
                    "Expected a variable name after 'create'",
                    token.line,
                    token.column,
                ))
            }
        };
        let declared_type = if self.take(&TokenKind::Colon) {
            Some(self.parse_type()?)
        } else {
            None
        };
        self.require(&TokenKind::Equal, "Expected '=' after variable name or type")?;
        let value = self.parse_expression()?;
        self.require(&TokenKind::Semicolon, "Expected ';' after variable declaration")?;
        Ok(Statement::Let {
            name,
            mutable,
            name_location: location,
            declared_type,
            value,
        })
    }
    fn parse_assignment(&mut self) -> Result<Statement, Diagnostic> {
        let token = self.advance();
        let location = Self::loc(&token);
        let name = match token.kind {
            TokenKind::Identifier(name) => name,
            _ => {
                return Err(Diagnostic::parsing(
                    "Expected a variable name after 'set'",
                    token.line,
                    token.column,
                ))
            }
        };
        self.require(&TokenKind::Equal, "Expected '=' after variable name")?;
        let value = self.parse_expression()?;
        self.require(&TokenKind::Semicolon, "Expected ';' after assignment")?;
        Ok(Statement::Assign { name, location, value })
    }
    fn parse_print(&mut self) -> Result<Statement, Diagnostic> {
        self.require(&TokenKind::LeftParen, "Expected '(' after print")?;
        let value = self.parse_expression()?;
        self.require(&TokenKind::RightParen, "Expected ')' after print value")?;
        self.require(&TokenKind::Semicolon, "Expected ';' after print")?;
        Ok(Statement::Print(value))
    }
    fn parse_return(&mut self, token: Token) -> Result<Statement, Diagnostic> {
        let value = self.parse_expression()?;
        self.require(&TokenKind::Semicolon, "Expected ';' after return value")?;
        Ok(Statement::Return {
            value,
            location: Self::loc(&token),
        })
    }
    fn parse_loop_control(&mut self) -> Result<Statement, Diagnostic> {
        let token = self.advance();
        let location = Self::loc(&token);
        self.require(&TokenKind::Semicolon, "Expected ';' after loop control statement")?;
        match token.kind {
            TokenKind::Break => Ok(Statement::Break { location }),
            TokenKind::Continue => Ok(Statement::Continue { location }),
            _ => unreachable!(),
        }
    }
    fn parse_function(&mut self) -> Result<Statement, Diagnostic> {
        let nt = self.advance();
        let loc = Self::loc(&nt);
        let name = match nt.kind {
            TokenKind::Identifier(n) => n,
            _ => {
                return Err(Diagnostic::parsing(
                    "Expected function name",
                    nt.line,
                    nt.column,
                ))
            }
        };
        self.require(&TokenKind::LeftParen, "Expected '(' after function name")?;
        let mut parameters = Vec::new();
        if !self.take(&TokenKind::RightParen) {
            loop {
                let t = self.advance();
                let parameter_location = Self::loc(&t);
                let p = match t.kind {
                    TokenKind::Identifier(n) => n,
                    _ => {
                        return Err(Diagnostic::parsing(
                            "Expected parameter name",
                            t.line,
                            t.column,
                        ))
                    }
                };
                self.require(&TokenKind::Colon, "Expected ':' after parameter name")?;
                let ty = self.parse_type()?;
                parameters.push((p, ty, parameter_location));
                if self.take(&TokenKind::RightParen) {
                    break;
                }
                self.require(&TokenKind::Comma, "Expected ',' or ')' after parameter")?;
            }
        }
        self.require(
            &TokenKind::Returns,
            "Expected 'returns' before function return type",
        )?;
        let rtloc = Self::loc(self.peek());
        let return_type = self.parse_type()?;
        self.require(&TokenKind::LeftBrace, "Expected '{' before function body")?;
        let body = self.parse_block_contents()?;
        Ok(Statement::Function {
            name,
            name_location: loc,
            parameters,
            return_type,
            return_type_location: rtloc,
            body,
        })
    }
    fn parse_type(&mut self) -> Result<PhonealType, Diagnostic> {
        let t = self.advance();
        match t.kind {
            TokenKind::Identifier(n) if n == "integer" => Ok(PhonealType::Integer),
            TokenKind::Identifier(n) if n == "text" => Ok(PhonealType::Text),
            TokenKind::Identifier(n) if n == "boolean" => Ok(PhonealType::Boolean),
            TokenKind::Identifier(n) => Err(Diagnostic::parsing(
                format!("Unknown type '{n}'; expected integer, text, or boolean"),
                t.line,
                t.column,
            )),
            _ => Err(Diagnostic::parsing(
                "Expected type name: integer, text, or boolean",
                t.line,
                t.column,
            )),
        }
    }
    fn parse_while(&mut self) -> Result<Statement, Diagnostic> {
        let start = self.tokens[self.position - 1].clone();
        self.require(&TokenKind::LeftParen, "Expected '(' after while")?;
        let condition = self.parse_expression()?;
        self.require(&TokenKind::RightParen, "Expected ')' after while condition")?;
        self.require(&TokenKind::LeftBrace, "Expected '{' after while condition")?;
        let body = self.parse_block_contents()?;
        Ok(Statement::While {
            condition,
            body,
            location: Self::loc(&start),
        })
    }
    fn parse_if(&mut self) -> Result<Statement, Diagnostic> {
        let start = self.tokens[self.position - 1].clone();
        self.require(&TokenKind::LeftParen, "Expected '(' after if")?;
        let condition = self.parse_expression()?;
        self.require(&TokenKind::RightParen, "Expected ')' after condition")?;
        self.require(&TokenKind::LeftBrace, "Expected '{' after if condition")?;
        let then_branch = self.parse_block_contents()?;
        let else_branch = if self.take(&TokenKind::Else) {
            if self.take(&TokenKind::If) {
                Some(vec![self.parse_if()?])
            } else {
                self.require(&TokenKind::LeftBrace, "Expected '{' after else")?;
                Some(self.parse_block_contents()?)
            }
        } else {
            None
        };
        Ok(Statement::If {
            condition,
            then_branch,
            else_branch,
            location: Self::loc(&start),
        })
    }
    fn parse_block(&mut self) -> Result<Statement, Diagnostic> {
        let t = self.tokens[self.position - 1].clone();
        let statements = self.parse_block_contents()?;
        Ok(Statement::Block {
            statements,
            location: Self::loc(&t),
        })
    }
    fn parse_block_contents(&mut self) -> Result<Vec<Statement>, Diagnostic> {
        if self.nesting_depth >= MAX_NESTING_DEPTH {
            let t = self.peek();
            return Err(Diagnostic::parsing(
                format!("Maximum parser nesting depth ({MAX_NESTING_DEPTH}) exceeded"),
                t.line,
                t.column,
            ));
        }
        self.nesting_depth += 1;
        let result = self.parse_block_contents_inner();
        self.nesting_depth -= 1;
        result
    }
    fn parse_block_contents_inner(&mut self) -> Result<Vec<Statement>, Diagnostic> {
        let mut out = Vec::new();
        while !matches!(self.peek().kind, TokenKind::RightBrace | TokenKind::Eof) {
            out.push(self.parse_statement()?);
        }
        self.require(&TokenKind::RightBrace, "Expected '}' to close block")?;
        Ok(out)
    }
    fn binary(
        &mut self,
        lower: fn(&mut Self) -> Result<Expression, Diagnostic>,
        ops: &[TokenKind],
    ) -> Result<Expression, Diagnostic> {
        let mut e = lower(self)?;
        while let Some(op) = ops.iter().find(|op| **op == self.peek().kind).cloned() {
            let t = self.advance();
            let right = lower(self)?;
            e = Expression::Binary {
                left: Box::new(e),
                operator: op,
                operator_location: Self::loc(&t),
                right: Box::new(right),
            };
        }
        Ok(e)
    }
    fn parse_expression(&mut self) -> Result<Expression, Diagnostic> {
        if self.nesting_depth >= MAX_NESTING_DEPTH {
            let t = self.peek();
            return Err(Diagnostic::parsing(
                format!("Maximum parser nesting depth ({MAX_NESTING_DEPTH}) exceeded"),
                t.line,
                t.column,
            ));
        }
        self.nesting_depth += 1;
        let result = self.parse_expression_inner();
        self.nesting_depth -= 1;
        result
    }
    fn parse_expression_inner(&mut self) -> Result<Expression, Diagnostic> {
        self.parse_or()
    }
    fn parse_or(&mut self) -> Result<Expression, Diagnostic> {
        self.binary(Self::parse_and, &[TokenKind::Or])
    }
    fn parse_and(&mut self) -> Result<Expression, Diagnostic> {
        self.binary(Self::parse_equality, &[TokenKind::And])
    }
    fn parse_equality(&mut self) -> Result<Expression, Diagnostic> {
        self.binary(
            Self::parse_comparison,
            &[TokenKind::EqualEqual, TokenKind::BangEqual],
        )
    }
    fn parse_comparison(&mut self) -> Result<Expression, Diagnostic> {
        self.binary(
            Self::parse_addition,
            &[
                TokenKind::Less,
                TokenKind::LessEqual,
                TokenKind::Greater,
                TokenKind::GreaterEqual,
            ],
        )
    }
    fn parse_addition(&mut self) -> Result<Expression, Diagnostic> {
        self.binary(
            Self::parse_multiplication,
            &[TokenKind::Plus, TokenKind::Minus],
        )
    }
    fn parse_multiplication(&mut self) -> Result<Expression, Diagnostic> {
        self.binary(
            Self::parse_unary,
            &[TokenKind::Star, TokenKind::Slash, TokenKind::Percent],
        )
    }
    fn parse_unary(&mut self) -> Result<Expression, Diagnostic> {
        if self.nesting_depth >= MAX_NESTING_DEPTH {
            let t = self.peek();
            return Err(Diagnostic::parsing(
                format!("Maximum parser nesting depth ({MAX_NESTING_DEPTH}) exceeded"),
                t.line,
                t.column,
            ));
        }
        self.nesting_depth += 1;
        let result = self.parse_unary_inner();
        self.nesting_depth -= 1;
        result
    }
    fn parse_unary_inner(&mut self) -> Result<Expression, Diagnostic> {
        if matches!(self.peek().kind, TokenKind::Not | TokenKind::Minus) {
            let t = self.advance();
            let operator_location = Self::loc(&t);
            let operator = t.kind;
            let operand = self.parse_unary()?;
            return Ok(Expression::Unary {
                operator,
                operator_location,
                operand: Box::new(operand),
            });
        }
        self.parse_primary()
    }
    fn parse_primary(&mut self) -> Result<Expression, Diagnostic> {
        let t = self.advance();
        let token_location = Self::loc(&t);
        match t.kind {
            TokenKind::Integer(v) => Ok(Expression::Integer(v)),
            TokenKind::String(v) => Ok(Expression::String(v)),
            TokenKind::True => Ok(Expression::Boolean(true)),
            TokenKind::False => Ok(Expression::Boolean(false)),
            TokenKind::Identifier(name) => {
                let loc = token_location;
                if self.take(&TokenKind::LeftParen) {
                    let mut args = Vec::new();
                    if !self.take(&TokenKind::RightParen) {
                        loop {
                            args.push(self.parse_expression()?);
                            if self.take(&TokenKind::RightParen) {
                                break;
                            }
                            self.require(&TokenKind::Comma, "Expected ',' or ')' after argument")?;
                        }
                    }
                    Ok(Expression::Call {
                        name,
                        location: loc,
                        arguments: args,
                    })
                } else {
                    Ok(Expression::Variable {
                        name,
                        location: loc,
                    })
                }
            }
            TokenKind::LeftParen => {
                let e = self.parse_expression()?;
                self.require(&TokenKind::RightParen, "Expected ')'")?;
                Ok(Expression::Group(Box::new(e)))
            }
            _ => Err(Diagnostic::parsing(
                "Expected an expression",
                t.line,
                t.column,
            )),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::Lexer;
    #[test]
    fn parses_english_function_syntax() {
        let s="function add(a: integer, b: integer) returns integer { return a + b; } create result = add(20,22);";
        let p = Parser::new(Lexer::new(s).scan().unwrap()).parse().unwrap();
        assert!(matches!(p[0], Statement::Function { .. }));
    }
    #[test]
    fn parses_mutable_and_set_statements() {
        let p = Parser::new(
            Lexer::new("create mutable n = 0; set n = 1;")
                .scan()
                .unwrap(),
        )
        .parse()
        .unwrap();
        assert!(matches!(&p[0], Statement::Let { mutable: true, .. }));
        assert!(matches!(&p[1], Statement::Assign { .. }));
    }
    #[test]
    fn parses_explicit_variable_type() {
        let parsed = Parser::new(Lexer::new("create age: integer = 18;").scan().unwrap())
            .parse()
            .unwrap();
        assert!(matches!(
            &parsed[0],
            Statement::Let {
                declared_type: Some(PhonealType::Integer),
                ..
            }
        ));
    }
    #[test]
    fn parses_if_else_and_logical_precedence() {
        let s = "if (true or false and not false) { print(1); } else { print(2); }";
        let p = Parser::new(Lexer::new(s).scan().unwrap()).parse().unwrap();
        assert!(matches!(p[0], Statement::If { .. }));
    }
    #[test]
    fn parses_while_and_loop_control() {
        let parsed = Parser::new(
            Lexer::new("while (true) { break; continue; }")
                .scan()
                .unwrap(),
        )
        .parse()
        .unwrap();
        assert!(matches!(parsed[0], Statement::While { .. }));
    }
    #[test]
    fn parses_nested_blocks() {
        let p = Parser::new(
            Lexer::new("{ create x = 1; { create y = 2; print(y); } }")
                .scan()
                .unwrap(),
        )
        .parse()
        .unwrap();
        assert!(matches!(p[0], Statement::Block { .. }));
    }
    #[test]
    fn rejects_excessively_nested_parentheses_without_panicking() {
        let source = format!(
            "print({}1{});",
            "(".repeat(MAX_NESTING_DEPTH + 10),
            ")".repeat(MAX_NESTING_DEPTH + 10)
        );
        let tokens = Lexer::new(&source).scan().unwrap();
        let error = Parser::new(tokens).parse().unwrap_err();
        assert!(error.message.contains("nesting depth"));
    }
    #[test]
    fn rejects_excessively_nested_blocks_without_panicking() {
        let source = format!(
            "{}print(1);{}",
            "{".repeat(MAX_NESTING_DEPTH + 10),
            "}".repeat(MAX_NESTING_DEPTH + 10)
        );
        let tokens = Lexer::new(&source).scan().unwrap();
        let error = Parser::new(tokens).parse().unwrap_err();
        assert!(error.message.contains("nesting depth"));
    }
    #[test]
    fn rejects_unclosed_block() {
        let e = Parser::new(Lexer::new("{ create x = 1;").scan().unwrap())
            .parse()
            .unwrap_err();
        assert!(e.message.contains("Expected '}'"));
    }
}
