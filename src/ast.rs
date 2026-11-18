use crate::token::TokenKind;

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    Integer(i64),
    String(String),
    Variable(String),
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
        value: Expression,
    },
    Print(Expression),
}
