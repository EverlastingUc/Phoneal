use crate::{diagnostic::SourceLocation, token::TokenKind, types::PhonealType};
#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    Integer(i64),
    String(String),
    Boolean(bool),
    Variable {
        name: String,
        location: SourceLocation,
    },
    Call {
        name: String,
        location: SourceLocation,
        arguments: Vec<Expression>,
    },
    Unary {
        operator: TokenKind,
        operator_location: SourceLocation,
        operand: Box<Expression>,
    },
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
    Let {
        name: String,
        name_location: SourceLocation,
        declared_type: Option<PhonealType>,
        value: Expression,
    },
    Print(Expression),
    Return {
        value: Expression,
        location: SourceLocation,
    },
    Function {
        name: String,
        name_location: SourceLocation,
        parameters: Vec<(String, PhonealType, SourceLocation)>,
        return_type: PhonealType,
        return_type_location: SourceLocation,
        body: Vec<Statement>,
    },
    Block {
        statements: Vec<Statement>,
        location: SourceLocation,
    },
    If {
        condition: Expression,
        then_branch: Vec<Statement>,
        else_branch: Option<Vec<Statement>>,
        location: SourceLocation,
    },
}
