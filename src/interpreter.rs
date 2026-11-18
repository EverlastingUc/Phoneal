use std::collections::HashMap;

use crate::{
    ast::{Expression, Statement},
    token::TokenKind,
};

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

pub fn evaluate(
    expression: &Expression,
    environment: &HashMap<String, Value>,
) -> Result<Value, String> {
    match expression {
        Expression::Integer(value) => Ok(Value::Integer(*value)),
        Expression::String(value) => Ok(Value::String(value.clone())),
        Expression::Variable(name) => environment
            .get(name)
            .cloned()
            .ok_or_else(|| format!("Undefined variable '{name}'")),
        Expression::Group(expression) => evaluate(expression, environment),
        Expression::Binary {
            left,
            operator,
            right,
        } => {
            let left_value = evaluate(left, environment)?;
            let right_value = evaluate(right, environment)?;

            match (left_value, operator, right_value) {
                (Value::Integer(left), TokenKind::Plus, Value::Integer(right)) => left
                    .checked_add(right)
                    .map(Value::Integer)
                    .ok_or_else(|| "Integer overflow".to_owned()),
                (Value::Integer(left), TokenKind::Minus, Value::Integer(right)) => left
                    .checked_sub(right)
                    .map(Value::Integer)
                    .ok_or_else(|| "Integer overflow".to_owned()),
                (Value::Integer(left), TokenKind::Star, Value::Integer(right)) => left
                    .checked_mul(right)
                    .map(Value::Integer)
                    .ok_or_else(|| "Integer overflow".to_owned()),
                (Value::Integer(_), TokenKind::Slash, Value::Integer(0)) => {
                    Err("Division by zero".to_owned())
                }
                (Value::Integer(left), TokenKind::Slash, Value::Integer(right)) => left
                    .checked_div(right)
                    .map(Value::Integer)
                    .ok_or_else(|| "Integer overflow".to_owned()),
                (Value::String(left), TokenKind::Plus, Value::String(right)) => {
                    Ok(Value::String(left + &right))
                }
                _ => Err("Operator used with incompatible types".to_owned()),
            }
        }
    }
}

pub fn execute(statements: &[Statement]) -> Result<Vec<String>, String> {
    let mut environment = HashMap::new();
    let mut output = Vec::new();

    for statement in statements {
        match statement {
            Statement::Let { name, value } => {
                if environment.contains_key(name) {
                    return Err(format!("Variable '{name}' already declared"));
                }

                let evaluated = evaluate(value, &environment)?;
                environment.insert(name.clone(), evaluated);
            }
            Statement::Print(expression) => {
                output.push(evaluate(expression, &environment)?.to_string());
            }
        }
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{lexer::Lexer, parser::Parser};

    fn run(source: &str) -> Result<Vec<String>, String> {
        let tokens = Lexer::new(source)
            .scan()
            .map_err(|error| error.to_string())?;
        let statements = Parser::new(tokens)
            .parse()
            .map_err(|error| error.to_string())?;
        execute(&statements)
    }

    #[test]
    fn prints_integer_and_string_values() {
        let output =
            run("let greeting = \"hello\"; let answer = 6 * 7; print(greeting); print(answer);")
                .unwrap();

        assert_eq!(output, vec!["hello", "42"]);
    }

    #[test]
    fn rejects_duplicate_variables() {
        let error = run("let value = 1; let value = 2;").unwrap_err();
        assert!(error.contains("already declared"));
    }

    #[test]
    fn rejects_undefined_variables() {
        let error = run("print(missing);").unwrap_err();
        assert!(error.contains("Undefined variable 'missing'"));
    }

    #[test]
    fn detects_division_by_zero() {
        let error = run("let result = 10 / 0;").unwrap_err();
        assert_eq!(error, "Division by zero");
    }

    #[test]
    fn detects_integer_overflow() {
        let error = run("let result = 9223372036854775807 + 1;").unwrap_err();
        assert_eq!(error, "Integer overflow");
    }

    #[test]
    fn concatenates_strings() {
        let output = run("let message = \"Phone\" + \"al\"; print(message);").unwrap();
        assert_eq!(output, vec!["Phoneal"]);
    }
}
