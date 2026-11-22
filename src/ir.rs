use crate::{
    ast::{Expression, Statement},
    diagnostic::{Diagnostic, DiagnosticStage, SourceLocation},
    token::TokenKind,
    types::PhonealType,
};
use std::collections::HashMap;

type TypeScope = HashMap<String, (PhonealType, bool)>;
type FunctionTable = HashMap<String, (Vec<PhonealType>, PhonealType)>;

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
        operator: TokenKind,
        operand: Box<TypedExpression>,
    },
    Binary {
        left: Box<TypedExpression>,
        operator: TokenKind,
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
            )
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
                let ty = match (operator, operand.ty) {
                    (TokenKind::Not, PhonealType::Boolean) => PhonealType::Boolean,
                    (TokenKind::Minus, PhonealType::Integer) => PhonealType::Integer,
                    _ => {
                        return Err(Self::error(
                            format!("Unary operator cannot be applied to {}", operand.ty),
                            Some(operator_location),
                        ))
                    }
                };
                (
                    TypedExpressionKind::Unary {
                        operator: operator.clone(),
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
                let ty = binary_type(operator.clone(), left.ty, right.ty).ok_or_else(|| {
                    Self::error(
                        format!("Operator cannot be applied to {} and {}", left.ty, right.ty),
                        Some(operator_location),
                    )
                })?;
                (
                    TypedExpressionKind::Binary {
                        left: Box::new(left),
                        operator: operator.clone(),
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

fn binary_type(operator: TokenKind, left: PhonealType, right: PhonealType) -> Option<PhonealType> {
    use PhonealType::{Boolean, Integer, Text};
    match (operator, left, right) {
        (TokenKind::Plus, Integer, Integer) => Some(Integer),
        (TokenKind::Plus, Text, Text) => Some(Text),
        (
            TokenKind::Minus | TokenKind::Star | TokenKind::Slash | TokenKind::Percent,
            Integer,
            Integer,
        ) => Some(Integer),
        (
            TokenKind::Less | TokenKind::LessEqual | TokenKind::Greater | TokenKind::GreaterEqual,
            Integer,
            Integer,
        ) => Some(Boolean),
        (TokenKind::EqualEqual | TokenKind::BangEqual, left, right) if left == right => {
            Some(Boolean)
        }
        (TokenKind::And | TokenKind::Or, Boolean, Boolean) => Some(Boolean),
        _ => None,
    }
}

/// Lowers a semantically valid AST into a compiler-independent typed representation.
/// Run semantic analysis first; this function repeats essential type and scope checks
/// so malformed ASTs cannot enter the IR.
pub fn lower(statements: &[Statement]) -> Result<TypedProgram, Diagnostic> {
    let program = Lowerer::new(statements).lower_program(statements)?;
    validate(&program)?;
    Ok(program)
}

/// Validates type consistency and control-flow placement in an already lowered program.
pub fn validate(program: &TypedProgram) -> Result<(), Diagnostic> {
    let signatures: HashMap<_, _> = program
        .functions
        .iter()
        .map(|function| {
            (
                function.name.clone(),
                (
                    function
                        .parameters
                        .iter()
                        .map(|parameter| parameter.ty)
                        .collect::<Vec<_>>(),
                    function.return_type,
                ),
            )
        })
        .collect();
    let mut globals = HashMap::new();
    for statement in &program.statements {
        if let TypedStatementKind::Let { name, mutable, ty, .. } = &statement.kind
        {
            if globals.insert(name.clone(), (*ty, *mutable)).is_some() {
                return Err(Lowerer::error(
                    "IR contains a duplicate top-level binding",
                    statement.location.as_ref(),
                ));
            }
        }
    }

    for function in &program.functions {
        let parameters: HashMap<_, _> = function
            .parameters
            .iter()
            .map(|parameter| (parameter.name.clone(), (parameter.ty, false)))
            .collect();
        if parameters.len() != function.parameters.len() {
            return Err(Lowerer::error(
                "IR function contains duplicate parameters",
                Some(&function.location),
            ));
        }
        let mut scopes = vec![globals.clone(), parameters];
        validate_statements(
            &function.body,
            &signatures,
            &mut scopes,
            Some(function.return_type),
            0,
            Some(1),
        )?;
    }

    let mut scopes = vec![HashMap::new()];
    validate_statements(&program.statements, &signatures, &mut scopes, None, 0, None)
}

fn validate_statements(
    statements: &[TypedStatement],
    signatures: &FunctionTable,
    scopes: &mut Vec<TypeScope>,
    return_type: Option<PhonealType>,
    loop_depth: usize,
    function_scope_depth: Option<usize>,
) -> Result<(), Diagnostic> {
    for statement in statements {
        let fail = |message: &str| Lowerer::error(message, statement.location.as_ref());
        match &statement.kind {
            TypedStatementKind::Let {
                name,
                mutable,
                ty,
                value,
            } => {
                validate_expression(value, signatures, scopes)?;
                if *ty != value.ty {
                    return Err(fail("IR declaration type does not match its initializer"));
                }
                let scope = scopes.last_mut().expect("IR validator always has a scope");
                if scope.insert(name.clone(), (*ty, *mutable)).is_some() {
                    return Err(fail("IR contains a duplicate binding in one scope"));
                }
            }
            TypedStatementKind::Assign { name, value } => {
                let (scope_index, (expected, mutable)) = scopes
                    .iter()
                    .enumerate()
                    .rev()
                    .find_map(|(index, scope)| {
                        scope.get(name).copied().map(|binding| (index, binding))
                    })
                    .ok_or_else(|| fail("IR assignment references an undefined binding"))?;
                if !mutable {
                    return Err(fail("IR assignment targets an immutable binding"));
                }
                if function_scope_depth.is_some_and(|depth| scope_index < depth) {
                    return Err(fail(
                        "IR function assignment targets an outer-scope binding",
                    ));
                }
                validate_expression(value, signatures, scopes)?;
                if value.ty != expected {
                    return Err(fail("IR assignment changes the binding type"));
                }
            }
            TypedStatementKind::Print(value) => {
                validate_expression(value, signatures, scopes)?;
            }
            TypedStatementKind::Return(value) => {
                validate_expression(value, signatures, scopes)?;
                let expected =
                    return_type.ok_or_else(|| fail("IR return appears outside a function"))?;
                if value.ty != expected {
                    return Err(fail("IR return type does not match its function"));
                }
            }
            TypedStatementKind::Block(body) => {
                scopes.push(HashMap::new());
                validate_statements(
                    body,
                    signatures,
                    scopes,
                    return_type,
                    loop_depth,
                    function_scope_depth,
                )?;
                scopes.pop();
            }
            TypedStatementKind::If {
                condition,
                then_branch,
                else_branch,
            } => {
                validate_expression(condition, signatures, scopes)?;
                if condition.ty != PhonealType::Boolean {
                    return Err(fail("IR if condition is not boolean"));
                }
                scopes.push(HashMap::new());
                validate_statements(
                    then_branch,
                    signatures,
                    scopes,
                    return_type,
                    loop_depth,
                    function_scope_depth,
                )?;
                scopes.pop();
                if let Some(branch) = else_branch {
                    scopes.push(HashMap::new());
                    validate_statements(
                        branch,
                        signatures,
                        scopes,
                        return_type,
                        loop_depth,
                        function_scope_depth,
                    )?;
                    scopes.pop();
                }
            }
            TypedStatementKind::While { condition, body } => {
                validate_expression(condition, signatures, scopes)?;
                if condition.ty != PhonealType::Boolean {
                    return Err(fail("IR while condition is not boolean"));
                }
                scopes.push(HashMap::new());
                validate_statements(
                    body,
                    signatures,
                    scopes,
                    return_type,
                    loop_depth + 1,
                    function_scope_depth,
                )?;
                scopes.pop();
            }
            TypedStatementKind::Break | TypedStatementKind::Continue if loop_depth == 0 => {
                return Err(fail("IR loop control appears outside a loop"));
            }
            TypedStatementKind::Break | TypedStatementKind::Continue => {}
        }
    }
    Ok(())
}

fn validate_expression(
    expression: &TypedExpression,
    signatures: &FunctionTable,
    scopes: &[TypeScope],
) -> Result<(), Diagnostic> {
    let fail = |message: &str| Lowerer::error(message, expression.location.as_ref());
    match &expression.kind {
        TypedExpressionKind::Integer(_) if expression.ty == PhonealType::Integer => Ok(()),
        TypedExpressionKind::Text(_) if expression.ty == PhonealType::Text => Ok(()),
        TypedExpressionKind::Boolean(_) if expression.ty == PhonealType::Boolean => Ok(()),
        TypedExpressionKind::Variable(name) => {
            let actual = scopes
                .iter()
                .rev()
                .find_map(|scope| scope.get(name).map(|(ty, _)| *ty))
                .ok_or_else(|| fail("IR variable references an undefined binding"))?;
            if actual != expression.ty {
                return Err(fail("IR variable type does not match its binding"));
            }
            Ok(())
        }
        TypedExpressionKind::Call { name, arguments } => {
            let (parameters, result) = signatures
                .get(name)
                .ok_or_else(|| fail("IR call references an unknown function"))?;
            if arguments.len() != parameters.len()
                || arguments
                    .iter()
                    .zip(parameters)
                    .any(|(argument, expected)| argument.ty != *expected)
                || expression.ty != *result
            {
                return Err(fail("IR function call signature is inconsistent"));
            }
            for argument in arguments {
                validate_expression(argument, signatures, scopes)?;
            }
            Ok(())
        }
        TypedExpressionKind::Unary { operator, operand } => {
            validate_expression(operand, signatures, scopes)?;
            let expected = match (operator, operand.ty) {
                (TokenKind::Not, PhonealType::Boolean) => PhonealType::Boolean,
                (TokenKind::Minus, PhonealType::Integer) => PhonealType::Integer,
                _ => return Err(fail("IR unary expression has incompatible operand")),
            };
            if expression.ty != expected {
                return Err(fail("IR unary expression has an invalid result type"));
            }
            Ok(())
        }
        TypedExpressionKind::Binary {
            left,
            operator,
            right,
        } => {
            validate_expression(left, signatures, scopes)?;
            validate_expression(right, signatures, scopes)?;
            if binary_type(operator.clone(), left.ty, right.ty) != Some(expression.ty) {
                return Err(fail(
                    "IR binary expression has incompatible operand or result types",
                ));
            }
            Ok(())
        }
        _ => Err(fail("IR literal has an invalid type")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{lexer::Lexer, parser::Parser, semantic::analyze};

    fn lower_source(source: &str) -> Result<TypedProgram, Diagnostic> {
        let tokens = Lexer::new(source).scan()?;
        let statements = Parser::new(tokens).parse()?;
        analyze(&statements)?;
        lower(&statements)
    }

    #[test]
    fn lowers_typed_declarations_expressions_and_functions() {
        let program = lower_source(
            "function add(a: integer, b: integer) returns integer { return a + b; } create result = add(20, 22); print(result);",
        )
        .unwrap();
        assert_eq!(program.functions.len(), 1);
        assert_eq!(
            program.functions[0].parameters[0].location,
            SourceLocation {
                line: 1,
                column: 14
            }
        );
        assert_eq!(program.statements.len(), 2);
        assert!(matches!(
            &program.statements[0].kind,
            TypedStatementKind::Let {
                ty: PhonealType::Integer,
                ..
            }
        ));
    }

    #[test]
    fn preserves_available_source_locations() {
        let program = lower_source("create value = 42; print(value);").unwrap();
        assert_eq!(
            program.statements[0].location,
            Some(SourceLocation { line: 1, column: 8 })
        );
        match &program.statements[0].kind {
            TypedStatementKind::Let { value, .. } => assert_eq!(
                value.location,
                Some(SourceLocation {
                    line: 1,
                    column: 16
                })
            ),
            _ => panic!("expected declaration"),
        }
        match &program.statements[1].kind {
            TypedStatementKind::Print(expression) => {
                assert_eq!(
                    expression.location,
                    Some(SourceLocation {
                        line: 1,
                        column: 26
                    })
                );
            }
            _ => panic!("expected print statement"),
        }
    }

    #[test]
    fn rejects_invalid_ir_types_and_loop_control() {
        let mut program = lower_source("while (true) { break; }").unwrap();
        program.statements = vec![TypedStatement {
            kind: TypedStatementKind::Break,
            location: None,
        }];
        assert!(validate(&program).is_err());

        let mut program = lower_source("print(1);").unwrap();
        if let TypedStatementKind::Print(expression) = &mut program.statements[0].kind {
            expression.ty = PhonealType::Text;
        }
        assert!(validate(&program).is_err());
    }

    #[test]
    fn validator_rejects_undefined_variables_and_immutable_assignments() {
        let mut program = lower_source("print(1);").unwrap();
        if let TypedStatementKind::Print(expression) = &mut program.statements[0].kind {
            expression.kind = TypedExpressionKind::Variable("missing".into());
        }
        assert!(validate(&program).is_err());

        let mut program = lower_source("create value = 1;").unwrap();
        let location = SourceLocation { line: 1, column: 1 };
        program.statements.push(TypedStatement {
            kind: TypedStatementKind::Assign {
                name: "value".into(),
                value: TypedExpression {
                    kind: TypedExpressionKind::Integer(2),
                    ty: PhonealType::Integer,
                    location: Some(location.clone()),
                },
            },
            location: Some(location),
        });
        assert!(validate(&program).is_err());
    }

    #[test]
    fn validator_rejects_function_mutation_of_outer_bindings() {
        let mut program = lower_source(
            "create mutable total = 0; function update() returns integer { return total; }",
        )
        .unwrap();
        let location = SourceLocation { line: 1, column: 1 };
        program.functions[0].body[0] = TypedStatement {
            kind: TypedStatementKind::Assign {
                name: "total".into(),
                value: TypedExpression {
                    kind: TypedExpressionKind::Integer(1),
                    ty: PhonealType::Integer,
                    location: Some(location.clone()),
                },
            },
            location: Some(location),
        };
        assert!(validate(&program).is_err());
    }

    #[test]
    fn lowers_continue_break_and_return_inside_nested_control_flow() {
        let program = lower_source(
            "function select(value: integer) returns integer { while (true) { if (value == 0) { continue; } break; } return value; }",
        )
        .unwrap();
        assert!(matches!(
            &program.functions[0].body[0].kind,
            TypedStatementKind::While { .. }
        ));
    }

    #[test]
    fn rejects_invalid_source_before_lowering() {
        assert!(lower_source("while (1) { print(1); }").is_err());
        assert!(lower_source("break;").is_err());
    }
}
