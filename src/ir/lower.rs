use super::{
    BinaryOperator, FunctionTable, TypeScope, TypedExpression, TypedExpressionKind, TypedFunction,
    TypedParameter, TypedProgram, TypedStatement, TypedStatementKind, UnaryOperator,
};
use crate::{
    ast::{Expression, Statement},
    diagnostic::{Diagnostic, DiagnosticStage, SourceLocation},
    token::TokenKind,
    types::PhonealType,
};
use std::collections::HashMap;

#[derive(Default)]
struct Lowerer {
    scopes: Vec<TypeScope>,
    functions: FunctionTable,
    loop_depth: usize,
    return_types: Vec<PhonealType>,
}

impl Lowerer {
    fn new(statements: &[Statement]) -> Self {
        let mut lowerer = Self {
            scopes: vec![HashMap::new()],
            ..Self::default()
        };
        for statement in statements {
            if let Statement::Function {
                name,
                parameters,
                return_type,
                ..
            } = statement
            {
                lowerer.functions.insert(
                    name.clone(),
                    (
                        parameters.iter().map(|(_, ty, _)| *ty).collect(),
                        *return_type,
                    ),
                );
            }
        }
        lowerer
    }

    fn error(message: impl Into<String>, location: Option<&SourceLocation>) -> Diagnostic {
        match location {
            Some(location) => Diagnostic::at(
                DiagnosticStage::Semantic,
                message,
                location.line,
                location.column,
            ),
            None => Diagnostic::semantic(message),
        }
    }

    fn lower_program(mut self, statements: &[Statement]) -> Result<TypedProgram, Diagnostic> {
        let mut functions = Vec::new();
        let mut body = Vec::new();

        for statement in statements {
            match statement {
                Statement::Function {
                    name,
                    name_location,
                    parameters,
                    return_type,
                    body: function_body,
                    ..
                } => {
                    let previous_loop_depth = std::mem::replace(&mut self.loop_depth, 0);
                    self.return_types.push(*return_type);
                    self.scopes.push(
                        parameters
                            .iter()
                            .map(|(name, ty, _)| (name.clone(), (*ty, false)))
                            .collect(),
                    );
                    let lowered_body = self.lower_statements(function_body)?;
                    self.scopes.pop();
                    self.return_types.pop();
                    self.loop_depth = previous_loop_depth;
                    functions.push(TypedFunction {
                        name: name.clone(),
                        parameters: parameters
                            .iter()
                            .map(|(name, ty, location)| TypedParameter {
                                name: name.clone(),
                                ty: *ty,
                                location: location.clone(),
                            })
                            .collect(),
                        return_type: *return_type,
                        body: lowered_body,
                        location: name_location.clone(),
                    });
                }
                _ => body.push(self.lower_statement(statement)?),
            }
        }

        Ok(TypedProgram {
            functions,
            statements: body,
        })
    }

    fn lower_statements(
        &mut self,
        statements: &[Statement],
    ) -> Result<Vec<TypedStatement>, Diagnostic> {
        statements
            .iter()
            .map(|statement| self.lower_statement(statement))
            .collect()
    }

    fn lower_statement(&mut self, statement: &Statement) -> Result<TypedStatement, Diagnostic> {
        let (kind, location) = match statement {
            Statement::Let {
                name,
                mutable,
                name_location,
                declared_type,
                value,
            } => {
                let value = self.lower_expression(value)?;
                let ty = declared_type.unwrap_or(value.ty);
                if ty != value.ty {
                    return Err(Self::error(
                        format!(
                            "Declaration of '{name}' expects {ty}, received {}",
                            value.ty
                        ),
                        Some(name_location),
                    ));
                }
                let scope = self.scopes.last_mut().expect("lowerer always has a scope");
                if scope.insert(name.clone(), (ty, *mutable)).is_some() {
                    return Err(Self::error(
                        format!("Variable '{name}' is already declared in this scope"),
                        Some(name_location),
                    ));
                }
                (
                    TypedStatementKind::Let {
                        name: name.clone(),
                        mutable: *mutable,
                        ty,
                        value,
                    },
                    Some(name_location.clone()),
                )
            }
            Statement::Assign {
                name,
                location,
                value,
            } => {
                let (expected, mutable) = self
                    .scopes
                    .iter()
                    .rev()
                    .find_map(|scope| scope.get(name).copied())
                    .ok_or_else(|| {
                        Self::error(
                            format!("Cannot assign to undefined variable '{name}'"),
                            Some(location),
                        )
                    })?;
                if !mutable {
                    return Err(Self::error(
                        format!("Cannot assign to immutable variable '{name}'"),
                        Some(location),
                    ));
                }
                let value = self.lower_expression(value)?;
                if expected != value.ty {
                    return Err(Self::error(
                        format!(
                            "Assignment to '{name}' expects {expected}, received {}",
                            value.ty
                        ),
                        Some(location),
                    ));
                }
                (
                    TypedStatementKind::Assign {
                        name: name.clone(),
                        value,
                    },
                    Some(location.clone()),
                )
            }
            Statement::Print(expression) => (
                TypedStatementKind::Print(self.lower_expression(expression)?),
                None,
            ),
            Statement::Return { value, location } => {
                let value = self.lower_expression(value)?;
                let expected = self.return_types.last().copied().ok_or_else(|| {
                    Self::error("Return is only valid inside a function", Some(location))
                })?;
                if value.ty != expected {
                    return Err(Self::error(
                        format!("Return expects {expected}, received {}", value.ty),
                        Some(location),
                    ));
                }
                (TypedStatementKind::Return(value), Some(location.clone()))
            }
            Statement::Block {
                statements,
                location,
            } => {
                self.scopes.push(HashMap::new());
                let statements = self.lower_statements(statements)?;
                self.scopes.pop();
                (
                    TypedStatementKind::Block(statements),
                    Some(location.clone()),
                )
            }
            Statement::If {
                condition,
                then_branch,
                else_branch,
                location,
            } => {
                let condition = self.lower_expression(condition)?;
                if condition.ty != PhonealType::Boolean {
                    return Err(Self::error("If condition must be boolean", Some(location)));
                }
                self.scopes.push(HashMap::new());
                let then_branch = self.lower_statements(then_branch)?;
                self.scopes.pop();
                let else_branch = if let Some(branch) = else_branch {
                    self.scopes.push(HashMap::new());
                    let lowered = self.lower_statements(branch)?;
                    self.scopes.pop();
                    Some(lowered)
                } else {
                    None
                };
                (
                    TypedStatementKind::If {
                        condition,
                        then_branch,
                        else_branch,
                    },
                    Some(location.clone()),
                )
            }
            Statement::While {
                condition,
                body,
                location,
            } => {
                let condition = self.lower_expression(condition)?;
                if condition.ty != PhonealType::Boolean {
                    return Err(Self::error(
                        "While condition must be boolean",
                        Some(location),
                    ));
                }
                self.loop_depth += 1;
                self.scopes.push(HashMap::new());
                let body = self.lower_statements(body)?;
                self.scopes.pop();
                self.loop_depth -= 1;
                (
                    TypedStatementKind::While { condition, body },
                    Some(location.clone()),
                )
            }
            Statement::Break { location } => {
                if self.loop_depth == 0 {
                    return Err(Self::error(
                        "'break' is only valid inside a loop",
                        Some(location),
                    ));
                }
                (TypedStatementKind::Break, Some(location.clone()))
            }
            Statement::Continue { location } => {
                if self.loop_depth == 0 {
                    return Err(Self::error(
                        "'continue' is only valid inside a loop",
                        Some(location),
                    ));
                }
                (TypedStatementKind::Continue, Some(location.clone()))
            }
            Statement::Function { .. } => {
                return Err(Self::error(
                    "Nested function declarations are not supported",
                    None,
                ));
            }
        };
        Ok(TypedStatement { kind, location })
    }

    fn lower_expression(&self, expression: &Expression) -> Result<TypedExpression, Diagnostic> {
        let (kind, ty, location) = match expression {
            Expression::Integer(value, location) => (
                TypedExpressionKind::Integer(*value),
                PhonealType::Integer,
                Some(location.clone()),
            ),
            Expression::String(value, location) => (
                TypedExpressionKind::Text(value.clone()),
                PhonealType::Text,
                Some(location.clone()),
            ),
            Expression::Boolean(value, location) => (
                TypedExpressionKind::Boolean(*value),
                PhonealType::Boolean,
                Some(location.clone()),
            ),
            Expression::Variable { name, location } => {
                let ty = self
                    .scopes
                    .iter()
                    .rev()
                    .find_map(|scope| scope.get(name).map(|(ty, _)| *ty))
                    .ok_or_else(|| {
                        Self::error(format!("Undefined variable '{name}'"), Some(location))
                    })?;
                (
                    TypedExpressionKind::Variable(name.clone()),
                    ty,
                    Some(location.clone()),
                )
            }
            Expression::Call {
                name,
                location,
                arguments,
            } => {
                let (parameters, return_type) =
                    self.functions.get(name).cloned().ok_or_else(|| {
                        Self::error(format!("Undefined function '{name}'"), Some(location))
                    })?;
                let arguments = arguments
                    .iter()
                    .map(|argument| self.lower_expression(argument))
                    .collect::<Result<Vec<_>, _>>()?;
                if arguments.len() != parameters.len() {
                    return Err(Self::error(
                        format!(
                            "Function '{name}' expects {} argument(s), received {}",
                            parameters.len(),
                            arguments.len()
                        ),
                        Some(location),
                    ));
                }
                for (index, (argument, expected)) in arguments.iter().zip(parameters).enumerate() {
                    if argument.ty != expected {
                        return Err(Self::error(
                            format!(
                                "Argument {} of '{name}' expects {expected}, received {}",
                                index + 1,
                                argument.ty
                            ),
                            Some(location),
                        ));
                    }
                }
                (
                    TypedExpressionKind::Call {
                        name: name.clone(),
                        arguments,
                    },
                    return_type,
                    Some(location.clone()),
                )
            }
            Expression::Group(inner) => return self.lower_expression(inner),
            Expression::Unary {
                operator,
                operator_location,
                operand,
            } => {
                let operand = self.lower_expression(operand)?;
                let operator = match operator {
                    TokenKind::Not => UnaryOperator::Not,
                    TokenKind::Minus => UnaryOperator::Negate,
                    _ => {
                        return Err(Self::error(
                            "Unsupported unary operator in AST",
                            Some(operator_location),
                        ))
                    }
                };
                let ty = match (operator, operand.ty) {
                    (UnaryOperator::Not, PhonealType::Boolean) => PhonealType::Boolean,
                    (UnaryOperator::Negate, PhonealType::Integer) => PhonealType::Integer,
                    _ => {
                        return Err(Self::error(
                            format!("Unary operator cannot be applied to {}", operand.ty),
                            Some(operator_location),
                        ))
                    }
                };
                (
                    TypedExpressionKind::Unary {
                        operator,
                        operand: Box::new(operand),
                    },
                    ty,
                    Some(operator_location.clone()),
                )
            }
            Expression::Binary {
                left,
                operator,
                operator_location,
                right,
            } => {
                let left = self.lower_expression(left)?;
                let right = self.lower_expression(right)?;
                let operator = lower_binary_operator(operator).ok_or_else(|| {
                    Self::error(
                        "Unsupported binary operator in AST",
                        Some(operator_location),
                    )
                })?;
                let ty = binary_type(operator, left.ty, right.ty).ok_or_else(|| {
                    Self::error(
                        format!("Operator cannot be applied to {} and {}", left.ty, right.ty),
                        Some(operator_location),
                    )
                })?;
                (
                    TypedExpressionKind::Binary {
                        left: Box::new(left),
                        operator,
                        right: Box::new(right),
                    },
                    ty,
                    Some(operator_location.clone()),
                )
            }
        };
        Ok(TypedExpression { kind, ty, location })
    }
}

fn lower_binary_operator(operator: &TokenKind) -> Option<BinaryOperator> {
    Some(match operator {
        TokenKind::Plus => BinaryOperator::Add,
        TokenKind::Minus => BinaryOperator::Subtract,
        TokenKind::Star => BinaryOperator::Multiply,
        TokenKind::Slash => BinaryOperator::Divide,
        TokenKind::Percent => BinaryOperator::Remainder,
        TokenKind::Less => BinaryOperator::Less,
        TokenKind::LessEqual => BinaryOperator::LessEqual,
        TokenKind::Greater => BinaryOperator::Greater,
        TokenKind::GreaterEqual => BinaryOperator::GreaterEqual,
        TokenKind::EqualEqual => BinaryOperator::Equal,
        TokenKind::BangEqual => BinaryOperator::NotEqual,
        TokenKind::And => BinaryOperator::And,
        TokenKind::Or => BinaryOperator::Or,
        _ => return None,
    })
}

pub(super) fn binary_type(
    operator: BinaryOperator,
    left: PhonealType,
    right: PhonealType,
) -> Option<PhonealType> {
    use BinaryOperator::*;
    use PhonealType::{Boolean, Integer, Text};
    match (operator, left, right) {
        (Add, Integer, Integer) => Some(Integer),
        (Add, Text, Text) => Some(Text),
        (Subtract | Multiply | Divide | Remainder, Integer, Integer) => Some(Integer),
        (Less | LessEqual | Greater | GreaterEqual, Integer, Integer) => Some(Boolean),
        (Equal | NotEqual, left, right) if left == right => Some(Boolean),
        (And | Or, Boolean, Boolean) => Some(Boolean),
        _ => None,
    }
}

/// Lowers a semantically valid AST into a compiler-independent typed representation.
/// Run semantic analysis first; this function repeats essential type and scope checks
/// so malformed ASTs cannot enter the IR.
pub fn lower(statements: &[Statement]) -> Result<TypedProgram, Diagnostic> {
    let program = Lowerer::new(statements).lower_program(statements)?;
    super::validate::validate(&program)?;
    Ok(program)
}

