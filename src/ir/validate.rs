use super::{
    lower::binary_type, FunctionTable, TypeScope, TypedExpression,
    TypedExpressionKind, TypedProgram, TypedStatement, TypedStatementKind, UnaryOperator,
};
use crate::{
    diagnostic::{Diagnostic, DiagnosticStage, SourceLocation},
    types::PhonealType,
};
use std::collections::HashMap;

fn validation_error(
    message: impl Into<String>,
    location: Option<&SourceLocation>,
) -> Diagnostic {
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

/// Validates type consistency and control-flow placement in an already lowered program.
pub fn validate(program: &TypedProgram) -> Result<(), Diagnostic> {
    let mut signatures = HashMap::new();
    for function in &program.functions {
        let signature = (
            function
                .parameters
                .iter()
                .map(|parameter| parameter.ty)
                .collect(),
            function.return_type,
        );
        if signatures
            .insert(function.name.clone(), signature)
            .is_some()
        {
            return Err(validation_error(
                "IR contains a duplicate function declaration",
                Some(&function.location),
            ));
        }
    }
    let mut globals = HashMap::new();
    for statement in &program.statements {
        if let TypedStatementKind::Let {
            name, mutable, ty, ..
        } = &statement.kind
        {
            if globals.insert(name.clone(), (*ty, *mutable)).is_some() {
                return Err(validation_error(
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
            return Err(validation_error(
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
        if !guarantees_return(&function.body) {
            return Err(validation_error(
                "IR function does not return a value on every path",
                Some(&function.location),
            ));
        }
    }

    let mut scopes = vec![HashMap::new()];
    validate_statements(&program.statements, &signatures, &mut scopes, None, 0, None)
}

fn guarantees_return(statements: &[TypedStatement]) -> bool {
    statements.iter().any(|statement| match &statement.kind {
        TypedStatementKind::Return(_) => true,
        TypedStatementKind::Block(body) => guarantees_return(body),
        TypedStatementKind::If {
            then_branch,
            else_branch: Some(other),
            ..
        } => guarantees_return(then_branch) && guarantees_return(other),
        _ => false,
    })
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
        let fail = |message: &str| validation_error(message, statement.location.as_ref());
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
    let fail = |message: &str| validation_error(message, expression.location.as_ref());
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
                (UnaryOperator::Not, PhonealType::Boolean) => PhonealType::Boolean,
                (UnaryOperator::Negate, PhonealType::Integer) => PhonealType::Integer,
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
            if binary_type(*operator, left.ty, right.ty) != Some(expression.ty) {
                return Err(fail(
                    "IR binary expression has incompatible operand or result types",
                ));
            }
            Ok(())
        }
        _ => Err(fail("IR literal has an invalid type")),
    }
}

