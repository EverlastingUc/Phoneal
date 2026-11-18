use crate::{diagnostic::SourceLocation, token::TokenKind};

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    Integer(i64),
    String(String),
    Variable {
        name: String,
        location: SourceLocation,
    },
    Binary {
        left: Box<Expression>,
        operator: TokenKind,
        right: Box<Expression>,
    },
    Group(Box<Expression>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Let {
        name: String,
        name_location: SourceLocation,
        value: Expression,
    },
    Print(Expression),
}
