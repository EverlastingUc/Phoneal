use crate::{diagnostic::SourceLocation, token::TokenKind, types::PhonealType};
#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    Integer(i64, SourceLocation),
    String(String, SourceLocation),
    Boolean(bool, SourceLocation),
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
        mutable: bool,
        name_location: SourceLocation,
        declared_type: Option<PhonealType>,
        value: Expression,
    },
    Assign {
        name: String,
        location: SourceLocation,
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
    While {
        condition: Expression,
        body: Vec<Statement>,
        location: SourceLocation,
    },
    Break {
        location: SourceLocation,
    },
    Continue {
        location: SourceLocation,
    },
}
