use crate::{
    ast::{Expression, Statement},
    diagnostic::{Diagnostic, DiagnosticStage, SourceLocation},
    token::TokenKind,
};
use std::collections::HashMap;
const MAX_CALL_DEPTH: usize = 128;
/// Maximum number of executed statements in one program run.
pub const MAX_EXECUTION_STEPS: usize = 50_000;
#[derive(Debug, Clone, PartialEq)]
pub enum Value {
    Integer(i64),
    String(String),
    Boolean(bool),
}
impl std::fmt::Display for Value {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Integer(v) => write!(f, "{v}"),
            Self::String(v) => write!(f, "{v}"),
            Self::Boolean(v) => write!(f, "{v}"),
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
fn runtime_error(m: impl Into<String>, l: &SourceLocation) -> Diagnostic {
    Diagnostic::at(DiagnosticStage::Runtime, m, l.line, l.column)
}
fn apply_binary(
    l: Value,
    op: &TokenKind,
    r: Value,
    loc: &SourceLocation,
) -> Result<Value, Diagnostic> {
    let fail = |m: &str| runtime_error(m, loc);
    match (l, op, r) {
        (Value::Integer(a), TokenKind::Plus, Value::Integer(b)) => a
            .checked_add(b)
            .map(Value::Integer)
            .ok_or_else(|| fail("Integer overflow")),
        (Value::Integer(a), TokenKind::Minus, Value::Integer(b)) => a
            .checked_sub(b)
            .map(Value::Integer)
            .ok_or_else(|| fail("Integer overflow")),
        (Value::Integer(a), TokenKind::Star, Value::Integer(b)) => a
            .checked_mul(b)
            .map(Value::Integer)
            .ok_or_else(|| fail("Integer overflow")),
        (Value::Integer(_), TokenKind::Slash, Value::Integer(0))
        | (Value::Integer(_), TokenKind::Percent, Value::Integer(0)) => {
            Err(fail("Division by zero"))
        }
        (Value::Integer(a), TokenKind::Slash, Value::Integer(b)) => a
            .checked_div(b)
            .map(Value::Integer)
            .ok_or_else(|| fail("Integer overflow")),
        (Value::Integer(a), TokenKind::Percent, Value::Integer(b)) => a
            .checked_rem(b)
            .map(Value::Integer)
            .ok_or_else(|| fail("Integer overflow")),
        (Value::String(a), TokenKind::Plus, Value::String(b)) => Ok(Value::String(a + &b)),
        (Value::Integer(a), TokenKind::Less, Value::Integer(b)) => Ok(Value::Boolean(a < b)),
        (Value::Integer(a), TokenKind::LessEqual, Value::Integer(b)) => Ok(Value::Boolean(a <= b)),
        (Value::Integer(a), TokenKind::Greater, Value::Integer(b)) => Ok(Value::Boolean(a > b)),
        (Value::Integer(a), TokenKind::GreaterEqual, Value::Integer(b)) => {
            Ok(Value::Boolean(a >= b))
        }
        (a, TokenKind::EqualEqual, b) => Ok(Value::Boolean(a == b)),
        (a, TokenKind::BangEqual, b) => Ok(Value::Boolean(a != b)),
        _ => Err(fail("Operator used with incompatible types")),
    }
}
fn lookup(env: &Environment, n: &str) -> Option<Value> {
    env.iter().rev().find_map(|s| s.get(n).cloned())
}
pub fn evaluate(e: &Expression, env: &HashMap<String, Value>) -> Result<Value, Diagnostic> {
    let scopes = vec![env.clone()];
    let mut out = Vec::new();
    let mut depth = 0;
    let mut steps = 0;
    evaluate_expression(e, &scopes, &HashMap::new(), &mut out, &mut depth, &mut steps)
}
fn evaluate_expression(
    e: &Expression,
    env: &Environment,
    funcs: &FunctionTable,
    out: &mut Vec<String>,
    depth: &mut usize,
    steps: &mut usize,
) -> Result<Value, Diagnostic> {
    match e {
        Expression::Integer(v) => Ok(Value::Integer(*v)),
        Expression::String(v) => Ok(Value::String(v.clone())),
        Expression::Boolean(v) => Ok(Value::Boolean(*v)),
        Expression::Variable { name, location } => lookup(env, name)
            .ok_or_else(|| runtime_error(format!("Undefined variable '{name}'"), location)),
        Expression::Group(inner) => evaluate_expression(inner, env, funcs, out, depth, steps),
        Expression::Unary {
            operator,
            operator_location,
            operand,
        } => {
            let v = evaluate_expression(operand, env, funcs, out, depth)?;
            match (operator, v) {
                (TokenKind::Not, Value::Boolean(b)) => Ok(Value::Boolean(!b)),
                (TokenKind::Minus, Value::Integer(i)) => i
                    .checked_neg()
                    .map(Value::Integer)
                    .ok_or_else(|| runtime_error("Integer overflow", operator_location)),
                _ => Err(runtime_error(
                    "Unary operator used with incompatible type",
                    operator_location,
                )),
            }
        }
        Expression::Binary {
            left,
            operator,
            operator_location,
            right,
        } => {
            let l = evaluate_expression(left, env, funcs, out, depth)?;
            if *operator == TokenKind::And && l == Value::Boolean(false) {
                return Ok(Value::Boolean(false));
            }
            if *operator == TokenKind::Or && l == Value::Boolean(true) {
                return Ok(Value::Boolean(true));
            }
            let r = evaluate_expression(right, env, funcs, out, depth)?;
            match operator {
                TokenKind::And => match (l, r) {
                    (Value::Boolean(a), Value::Boolean(b)) => Ok(Value::Boolean(a && b)),
                    _ => Err(runtime_error(
                        "Logical operator requires boolean operands",
                        operator_location,
                    )),
                },
                TokenKind::Or => match (l, r) {
                    (Value::Boolean(a), Value::Boolean(b)) => Ok(Value::Boolean(a || b)),
                    _ => Err(runtime_error(
                        "Logical operator requires boolean operands",
                        operator_location,
                    )),
                },
                _ => apply_binary(l, operator, r, operator_location),
            }
        }
        Expression::Call {
            name,
            location,
            arguments,
        } => {
            let f = funcs
                .get(name)
                .ok_or_else(|| runtime_error(format!("Undefined function '{name}'"), location))?;
            if f.parameters.len() != arguments.len() {
                return Err(runtime_error(
                    format!(
                        "Function '{name}' expects {} argument(s), received {}",
                        f.parameters.len(),
                        arguments.len()
                    ),
                    location,
                ));
            }
            if *depth >= MAX_CALL_DEPTH {
                return Err(runtime_error(
                    format!("Maximum function call depth ({MAX_CALL_DEPTH}) exceeded"),
                    location,
                ));
            }
            let mut vals = Vec::new();
            for a in arguments {
                vals.push(evaluate_expression(a, env, funcs, out, depth)?);
            }
            let globals = env.first().cloned().unwrap_or_default();
            let mut local = HashMap::new();
            for (p, v) in f.parameters.iter().zip(vals) {
                local.insert(p.clone(), v);
            }
            let mut call_env = vec![globals, local];
            *depth += 1;
            let result = execute_statements(&f.body, &mut call_env, funcs, out, depth);
            *depth -= 1;
            match result? {
                Some(v) => Ok(v),
                None => Err(runtime_error(
                    format!("Function '{name}' completed without returning a value"),
                    location,
                )),
            }
        }
    }
}
fn execute_statements(
    ss: &[Statement],
    env: &mut Environment,
    funcs: &FunctionTable,
    out: &mut Vec<String>,
    depth: &mut usize,
    steps: &mut usize,
) -> Result<Option<Value>, Diagnostic> {
    for s in ss {
        if *steps >= MAX_EXECUTION_STEPS {
            return Err(Diagnostic::runtime(format!("Maximum execution step budget ({MAX_EXECUTION_STEPS}) exceeded")));
        }
        *steps += 1;
        match s {
            Statement::Let {
                name,
                name_location,
                value,
            } => {
                if env.last().unwrap().contains_key(name) {
                    return Err(runtime_error(
                        format!("Variable '{name}' already declared in this scope"),
                        name_location,
                    ));
                }
                let v = evaluate_expression(value, env, funcs, out, depth)?;
                env.last_mut().unwrap().insert(name.clone(), v);
            }
            Statement::Print(e) => {
                let v = evaluate_expression(e, env, funcs, out, depth)?;
                out.push(v.to_string());
            }
            Statement::Return { value, .. } => {
                return Ok(Some(evaluate_expression(value, env, funcs, out, depth)?))
            }
            Statement::Function { .. } => {}
            Statement::Block { statements, .. } => {
                env.push(HashMap::new());
                let r = execute_statements(statements, env, funcs, out, depth);
                env.pop();
                if let Some(v) = r? {
                    return Ok(Some(v));
                }
            }
            Statement::If {
                condition,
                then_branch,
                else_branch,
                location,
            } => {
                let c = evaluate_expression(condition, env, funcs, out, depth)?;
                let branch = match c {
                    Value::Boolean(true) => Some(then_branch),
                    Value::Boolean(false) => else_branch.as_ref(),
                    _ => {
                        return Err(runtime_error(
                            "If condition must evaluate to boolean",
                            location,
                        ))
                    }
                };
                if let Some(b) = branch {
                    env.push(HashMap::new());
                    let r = execute_statements(b, env, funcs, out, depth);
                    env.pop();
                    if let Some(v) = r? {
                        return Ok(Some(v));
                    }
                }
            }
        }
    }
    Ok(None)
}
pub fn execute(ss: &[Statement]) -> Result<Vec<String>, Diagnostic> {
    let mut funcs = HashMap::new();
    for s in ss {
        if let Statement::Function {
            name,
            parameters,
            body,
            ..
        } = s
        {
            funcs.insert(
                name.clone(),
                RuntimeFunction {
                    parameters: parameters.iter().map(|(p, _, _)| p.clone()).collect(),
                    body: body.clone(),
                },
            );
        }
    }
    let mut env = vec![HashMap::new()];
    let mut out = Vec::new();
    let mut depth = 0;
    let mut steps = 0;
    match execute_statements(ss, &mut env, &funcs, &mut out, &mut depth, &mut steps)? {
        Some(_) => Err(Diagnostic::runtime(
            "Return is only valid inside a function",
        )),
        None => Ok(out),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{lexer::Lexer, parser::Parser, semantic::analyze_all};
    fn run(s: &str) -> Result<Vec<String>, String> {
        let ts = Lexer::new(s).scan().map_err(|e| e.to_string())?;
        let ss = Parser::new(ts).parse().map_err(|e| e.to_string())?;
        let ds = analyze_all(&ss);
        if let Some(e) = ds.iter().next() {
            return Err(e.to_string());
        }
        execute(&ss).map_err(|e| e.to_string())
    }
    #[test]
    fn prints_values() {
        assert_eq!(run("create greeting = \"hello\"; create answer = 6 * 7; print(greeting); print(answer);").unwrap(),["hello","42"]);
    }
    #[test]
    fn runs_boolean_conditions() {
        assert_eq!(run("create age = 20; if (age >= 18 and true) { print(\"adult\"); } else { print(\"minor\"); }").unwrap(),["adult"]);
    }
    #[test]
    fn short_circuits_and_or() {
        assert_eq!(run("if (false and 1 / 0 == 0) { print(1); } else { print(2); } if (true or 1 / 0 == 0) { print(3); }").unwrap(),["2","3"]);
    }
    #[test]
    fn evaluates_not_and_comparisons() {
        assert_eq!(
            run("create ok = not (3 > 5); print(ok); print(3 != 4);").unwrap(),
            ["true", "true"]
        );
    }
    #[test]
    fn typed_function_calls_work() {
        assert_eq!(run("function add(a: integer, b: integer) returns integer { return a + b; } print(add(20,22));").unwrap(),["42"]);
    }
    #[test]
    fn recursive_factorial_works() {
        assert_eq!(run("function factorial(n: integer) returns integer { if (n <= 1) { return 1; } else { return n * factorial(n - 1); } } print(factorial(5));").unwrap(),["120"]);
    }
    #[test]
    fn reports_division_by_zero() {
        assert!(run("create n = 10 / 0;")
            .unwrap_err()
            .contains("Division by zero"));
    }
    #[test]
    fn reports_recursion_limit() {
        assert!(run(
            "function repeat(n: integer) returns integer { return repeat(n); } print(repeat(1));"
        )
        .unwrap_err()
        .contains("Maximum function call depth"));
    }
}
