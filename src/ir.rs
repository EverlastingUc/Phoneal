use crate::{diagnostic::SourceLocation, types::PhonealType};
use std::collections::HashMap;

pub(super) type TypeScope = HashMap<String, (PhonealType, bool)>;
pub(super) type FunctionTable = HashMap<String, (Vec<PhonealType>, PhonealType)>;

#[derive(Debug, Clone, PartialEq)]
pub struct TypedProgram {
    pub functions: Vec<TypedFunction>,
    pub statements: Vec<TypedStatement>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedParameter {
    pub name: String,
    pub ty: PhonealType,
    pub location: SourceLocation,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedFunction {
    pub name: String,
    pub parameters: Vec<TypedParameter>,
    pub return_type: PhonealType,
    pub body: Vec<TypedStatement>,
    pub location: SourceLocation,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedExpression {
    pub kind: TypedExpressionKind,
    pub ty: PhonealType,
    pub location: Option<SourceLocation>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOperator {
    Negate,
    Not,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOperator {
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
    Equal,
    NotEqual,
    And,
    Or,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypedExpressionKind {
    Integer(i64),
    Text(String),
    Boolean(bool),
    Variable(String),
    Call {
        name: String,
        arguments: Vec<TypedExpression>,
    },
    Unary {
        operator: UnaryOperator,
        operand: Box<TypedExpression>,
    },
    Binary {
        left: Box<TypedExpression>,
        operator: BinaryOperator,
        right: Box<TypedExpression>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedStatement {
    pub kind: TypedStatementKind,
    pub location: Option<SourceLocation>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TypedStatementKind {
    Let {
        name: String,
        mutable: bool,
        ty: PhonealType,
        value: TypedExpression,
    },
    Assign {
        name: String,
        value: TypedExpression,
    },
    Print(TypedExpression),
    Return(TypedExpression),
    Block(Vec<TypedStatement>),
    If {
        condition: TypedExpression,
        then_branch: Vec<TypedStatement>,
        else_branch: Option<Vec<TypedStatement>>,
    },
    While {
        condition: TypedExpression,
        body: Vec<TypedStatement>,
    },
    Break,
    Continue,
}


mod lower;
mod validate;

#[cfg(test)]
mod tests;

pub use lower::lower;
pub use validate::validate;
