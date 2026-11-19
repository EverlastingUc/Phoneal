use crate::{diagnostic::SourceLocation, token::TokenKind};

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    Integer(i64),
    String(String),
    Variable { name: String, location: SourceLocation },
    Call { name: String, location: SourceLocation, arguments: Vec<Expression> },
    Binary {
        left: Box<Expression>,
        operator: TokenKind,
        operator_location: SourceLocation,
        right: Box<Expression>,
    },
    Group(Box<Expression>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Let { name: String, name_location: SourceLocation, value: Expression },
    Print(Expression),
    Return { value: Expression, location: SourceLocation },
    Function {
        name: String,
        name_location: SourceLocation,
        parameters: Vec<(String, SourceLocation)>,
        body: Vec<Statement>,
    },
    Block { statements: Vec<Statement>, location: SourceLocation },
}
