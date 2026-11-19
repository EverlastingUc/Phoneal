use std::collections::HashMap;

use crate::{
    ast::{Expression, Statement},
    diagnostic::{Diagnostic, DiagnosticStage, SourceLocation},
    token::TokenKind,
};

const MAX_CALL_DEPTH: usize = 128;

#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Integer(i64),
    String(String),
}

impl std::fmt::Display for Value {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Integer(value) => write!(formatter, "{value}"),
            Self::String(value) => write!(formatter, "{value}"),
        }
    }
}

type Environment = Vec<HashMap<String, Value>>;
#[derive(Clone)]
struct RuntimeFunction {
    parameters: Vec<String>,
    body: Vec<Statement>,
}
type FunctionTable = HashMap<String, RuntimeFunction>;

fn runtime_error(message: impl Into<String>, location: &SourceLocation) -> Diagnostic {
    Diagnostic::at(DiagnosticStage::Runtime, message, location.line, location.column)
}

fn apply_binary(
    left: Value,
    operator: &TokenKind,
    right: Value,
    location: &SourceLocation,
) -> Result<Value, Diagnostic> {
    let failure = |message: &str| runtime_error(message, location);
    match (left, operator, right) {
        (Value::Integer(left), TokenKind::Plus, Value::Integer(right)) =>
            left.checked_add(right).map(Value::Integer).ok_or_else(|| failure("Integer overflow")),
        (Value::Integer(left), TokenKind::Minus, Value::Integer(right)) =>
            left.checked_sub(right).map(Value::Integer).ok_or_else(|| failure("Integer overflow")),
        (Value::Integer(left), TokenKind::Star, Value::Integer(right)) =>
            left.checked_mul(right).map(Value::Integer).ok_or_else(|| failure("Integer overflow")),
        (Value::Integer(_), TokenKind::Slash, Value::Integer(0)) => Err(failure("Division by zero")),
        (Value::Integer(left), TokenKind::Slash, Value::Integer(right)) =>
            left.checked_div(right).map(Value::Integer).ok_or_else(|| failure("Integer overflow")),
        (Value::String(left), TokenKind::Plus, Value::String(right)) => Ok(Value::String(left + &right)),
        _ => Err(failure("Operator used with incompatible types")),
    }
}

fn lookup(environment: &Environment, name: &str) -> Option<Value> {
    environment.iter().rev().find_map(|scope| scope.get(name).cloned())
}

pub fn evaluate(expression: &Expression, environment: &HashMap<String, Value>) -> Result<Value, Diagnostic> {
    let scopes = vec![environment.clone()];
    let mut output = Vec::new();
    let mut depth = 0;
    evaluate_expression(expression, &scopes, &HashMap::new(), &mut output, &mut depth)
}

fn evaluate_expression(
    expression: &Expression,
    environment: &Environment,
    functions: &FunctionTable,
    output: &mut Vec<String>,
    depth: &mut usize,
) -> Result<Value, Diagnostic> {
    match expression {
        Expression::Integer(value) => Ok(Value::Integer(*value)),
        Expression::String(value) => Ok(Value::String(value.clone())),
        Expression::Variable { name, location } => lookup(environment, name)
            .ok_or_else(|| runtime_error(format!("Undefined variable '{name}'"), location)),
        Expression::Group(inner) => evaluate_expression(inner, environment, functions, output, depth),
        Expression::Binary { left, operator, operator_location, right } => {
            let left = evaluate_expression(left, environment, functions, output, depth)?;
            let right = evaluate_expression(right, environment, functions, output, depth)?;
            apply_binary(left, operator, right, operator_location)
        }
        Expression::Call { name, location, arguments } => {
            let function = functions.get(name)
                .ok_or_else(|| runtime_error(format!("Undefined function '{name}'"), location))?;
            if function.parameters.len() != arguments.len() {
                return Err(runtime_error(
                    format!("Function '{name}' expects {} argument(s), received {}", function.parameters.len(), arguments.len()),
                    location,
                ));
            }
            if *depth >= MAX_CALL_DEPTH {
                return Err(runtime_error(format!("Maximum function call depth ({MAX_CALL_DEPTH}) exceeded"), location));
            }

            let mut values = Vec::with_capacity(arguments.len());
            for argument in arguments {
                values.push(evaluate_expression(argument, environment, functions, output, depth)?);
            }

            let global_scope = environment.first().cloned().unwrap_or_default();
            let mut local_scope = HashMap::new();
            for (parameter, value) in function.parameters.iter().zip(values) {
                local_scope.insert(parameter.clone(), value);
            }
            let mut call_environment = vec![global_scope, local_scope];
            *depth += 1;
            let result = execute_statements(&function.body, &mut call_environment, functions, output, depth);
            *depth -= 1;
            match result? {
                Some(value) => Ok(value),
                None => Err(runtime_error(format!("Function '{name}' completed without returning a value"), location)),
            }
        }
    }
}

fn execute_statements(
    statements: &[Statement],
    environment: &mut Environment,
    functions: &FunctionTable,
    output: &mut Vec<String>,
    depth: &mut usize,
) -> Result<Option<Value>, Diagnostic> {
    for statement in statements {
        match statement {
            Statement::Let { name, name_location, value } => {
                if environment.last().expect("scope stack is never empty").contains_key(name) {
                    return Err(runtime_error(format!("Variable '{name}' already declared in this scope"), name_location));
                }
                let evaluated = evaluate_expression(value, environment, functions, output, depth)?;
                environment.last_mut().expect("scope stack is never empty").insert(name.clone(), evaluated);
            }
            Statement::Print(expression) => {
                let value = evaluate_expression(expression, environment, functions, output, depth)?;
                output.push(value.to_string());
            }
            Statement::Return { value, .. } => {
                return Ok(Some(evaluate_expression(value, environment, functions, output, depth)?));
            }
            Statement::Function { .. } => {}
            Statement::Block { statements, .. } => {
                environment.push(HashMap::new());
                let result = execute_statements(statements, environment, functions, output, depth);
                environment.pop();
                if let Some(value) = result? { return Ok(Some(value)); }
            }
        }
    }
    Ok(None)
}

pub fn execute(statements: &[Statement]) -> Result<Vec<String>, Diagnostic> {
    let mut functions = HashMap::new();
    for statement in statements {
        if let Statement::Function { name, parameters, body, .. } = statement {
            functions.insert(name.clone(), RuntimeFunction {
                parameters: parameters.iter().map(|(parameter, _)| parameter.clone()).collect(),
                body: body.clone(),
            });
        }
    }

    let mut environment = vec![HashMap::new()];
    let mut output = Vec::new();
    let mut depth = 0;
    match execute_statements(statements, &mut environment, &functions, &mut output, &mut depth)? {
        Some(_) => Err(Diagnostic::runtime("Return is only valid inside a function")),
        None => Ok(output),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{lexer::Lexer, parser::Parser, semantic::analyze_all};

    fn run(source: &str) -> Result<Vec<String>, String> {
        let tokens = Lexer::new(source).scan().map_err(|error| error.to_string())?;
        let statements = Parser::new(tokens).parse().map_err(|error| error.to_string())?;
        let diagnostics = analyze_all(&statements);
        if let Some(error) = diagnostics.iter().next() { return Err(error.to_string()); }
        execute(&statements).map_err(|error| error.to_string())
    }

    #[test]
    fn prints_integer_and_string_values() {
        let output = run("let greeting = \"hello\"; let answer = 6 * 7; print(greeting); print(answer);").unwrap();
        assert_eq!(output, vec!["hello", "42"]);
    }

    #[test]
    fn executes_functions_and_returns_values() {
        let output = run("fn add(a, b) { return a + b; } let result = add(20, 22); print(result);").unwrap();
        assert_eq!(output, ["42"]);
    }

    #[test]
    fn supports_nested_function_calls() {
        let output = run("fn double(x) { return x * 2; } fn add(a, b) { return a + b; } print(add(double(5), double(6)));").unwrap();
        assert_eq!(output, ["22"]);
    }

    #[test]
    fn rejects_runtime_call_depth_exhaustion() {
        let source = "fn loop(n) { return loop(n); } print(loop(1));";
        let error = run(source).unwrap_err();
        assert!(error.contains("Maximum function call depth"));
    }

    #[test]
    fn detects_division_by_zero() {
        let error = run("let result = 10 / 0;").unwrap_err();
        assert_eq!(error, "runtime error: Division by zero at 1:17");
    }

    #[test]
    fn detects_integer_overflow() {
        let error = run("let result = 9223372036854775807 + 1;").unwrap_err();
        assert_eq!(error, "runtime error: Integer overflow at 1:34");
    }

    #[test]
    fn nested_blocks_create_and_remove_scopes() {
        let output = run("let item = 1; { let item = 2; print(item); } print(item);").unwrap();
        assert_eq!(output, ["2", "1"]);
    }
}
