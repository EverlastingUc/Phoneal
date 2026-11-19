use std::collections::HashMap;

use crate::{
    ast::{Expression, Statement},
    diagnostic::{Diagnostic, DiagnosticStage},
    token::TokenKind,
    types::PhonealType,
};

/// Validates names and expression types before interpretation.
/// The current language has one program-wide scope.
pub fn analyze(statements: &[Statement]) -> Result<(), Diagnostic> {
    analyze_all(statements).into_result()
}

/// Runs semantic analysis and returns every independent diagnostic found.
pub fn analyze_all(statements: &[Statement]) -> DiagnosticSet {
    let mut symbols: HashMap<String, Option<PhonealType>> = HashMap::new();
    let mut diagnostics = Vec::new();

    for statement in statements {
        match statement {
            Statement::Let {
                name,
                name_location,
                value,
            } => {
                let already_declared = symbols.contains_key(name);
                if already_declared {
                    diagnostics.push(Diagnostic::at(
                        DiagnosticStage::Semantic,
                        format!("Variable '{name}' is already declared"),
                        name_location.line,
                        name_location.column,
                    ));
                }

                let inferred = infer_expression(value, &symbols, &mut diagnostics);
                if !already_declared {
                    symbols.insert(name.clone(), inferred);
                }
            }
            Statement::Print(expression) => {
                infer_expression(expression, &symbols, &mut diagnostics);
            }
        }
    }

    DiagnosticSet { diagnostics }
}

/// A collection of diagnostics produced by one analysis pass.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticSet {
    diagnostics: Vec<Diagnostic>,
}

impl DiagnosticSet {
    pub fn is_empty(&self) -> bool {
        self.diagnostics.is_empty()
    }

    pub fn len(&self) -> usize {
        self.diagnostics.len()
    }

    pub fn iter(&self) -> impl Iterator<Item = &Diagnostic> {
        self.diagnostics.iter()
    }

    pub fn into_vec(self) -> Vec<Diagnostic> {
        self.diagnostics
    }

    pub fn into_result(self) -> Result<(), Diagnostic> {
        match self.diagnostics.into_iter().next() {
            Some(diagnostic) => Err(diagnostic),
            None => Ok(()),
        }
    }
}

fn infer_expression(
    expression: &Expression,
    symbols: &HashMap<String, Option<PhonealType>>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<PhonealType> {
    match expression {
        Expression::Integer(_) => Some(PhonealType::Integer),
        Expression::String(_) => Some(PhonealType::String),
        Expression::Variable { name, location } => match symbols.get(name) {
            Some(value_type) => *value_type,
            None => {
                diagnostics.push(Diagnostic::at(
                    DiagnosticStage::Semantic,
                    format!("Undefined variable '{name}'"),
                    location.line,
                    location.column,
                ));
                None
            }
        },
        Expression::Group(inner) => infer_expression(inner, symbols, diagnostics),
        Expression::Binary {
            left,
            operator,
            operator_location,
            right,
        } => {
            let left_type = infer_expression(left, symbols, diagnostics);
            let right_type = infer_expression(right, symbols, diagnostics);
            let (Some(left_type), Some(right_type)) = (left_type, right_type) else {
                return None;
            };

            let result = match (operator, left_type, right_type) {
                (TokenKind::Plus, PhonealType::Integer, PhonealType::Integer) => {
                    Some(PhonealType::Integer)
                }
                (TokenKind::Plus, PhonealType::String, PhonealType::String) => {
                    Some(PhonealType::String)
                }
                (
                    TokenKind::Minus | TokenKind::Star | TokenKind::Slash,
                    PhonealType::Integer,
                    PhonealType::Integer,
                ) => Some(PhonealType::Integer),
                _ => None,
            };

            if result.is_none() {
                let symbol = match operator {
                    TokenKind::Plus => "+",
                    TokenKind::Minus => "-",
                    TokenKind::Star => "*",
                    TokenKind::Slash => "/",
                    _ => "?",
                };
                diagnostics.push(Diagnostic::at(
                    DiagnosticStage::Semantic,
                    format!(
                        "Operator '{symbol}' cannot be applied to {left_type} and {right_type}"
                    ),
                    operator_location.line,
                    operator_location.column,
                ));
            }
            result
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{compile, diagnostic::DiagnosticStage, lexer::Lexer, parser::Parser};

    fn statements(source: &str) -> Vec<Statement> {
        Parser::new(Lexer::new(source).scan().unwrap())
            .parse()
            .unwrap()
    }

    #[test]
    fn accepts_integer_and_string_expressions() {
        assert!(analyze(&statements(
            "let n = 2 + 3 * 4; let s = \"a\" + \"b\"; print(n); print(s);"
        ))
        .is_ok());
    }

    #[test]
    fn rejects_undefined_variable_with_location() {
        let error = analyze(&statements("print(missing);")).unwrap_err();
        assert_eq!(error.stage, DiagnosticStage::Semantic);
        assert_eq!(error.message, "Undefined variable 'missing'");
        assert_eq!(error.location.unwrap().column, 7);
    }

    #[test]
    fn rejects_duplicate_declarations_at_second_name() {
        let error = analyze(&statements("let value = 1; let value = 2;")).unwrap_err();
        assert_eq!(error.message, "Variable 'value' is already declared");
        assert_eq!(error.location.unwrap().column, 20);
    }

    #[test]
    fn declaration_is_not_visible_inside_its_initializer() {
        let error = analyze(&statements("let value = value + 1;")).unwrap_err();
        assert_eq!(error.message, "Undefined variable 'value'");
        assert_eq!(error.location.unwrap().column, 13);
    }

    #[test]
    fn reports_multiple_independent_semantic_errors() {
        let result = analyze_all(&statements(
            "print(first); let value = missing + other; print(last);",
        ));
        assert_eq!(result.len(), 4);
        let messages: Vec<_> = result.iter().map(|item| item.message.as_str()).collect();
        assert_eq!(
            messages,
            [
                "Undefined variable 'first'",
                "Undefined variable 'missing'",
                "Undefined variable 'other'",
                "Undefined variable 'last'",
            ]
        );
    }

    #[test]
    fn duplicate_initializer_is_still_checked_without_replacing_original() {
        let result = analyze_all(&statements(
            "let value = 1; let value = missing; print(value);",
        ));
        assert_eq!(result.len(), 2);
        assert_eq!(
            result.iter().next().unwrap().message,
            "Variable 'value' is already declared"
        );
        assert_eq!(
            result.iter().nth(1).unwrap().message,
            "Undefined variable 'missing'"
        );
    }

    #[test]
    fn rejects_mixed_type_addition() {
        let error = analyze(&statements("let result = 1 + \"two\";")).unwrap_err();
        assert_eq!(
            error.message,
            "Operator '+' cannot be applied to integer and string"
        );
        assert_eq!(error.location.unwrap().column, 18);
    }

    #[test]
    fn rejects_string_subtraction() {
        let error = analyze(&statements("print(\"a\" - \"b\");")).unwrap_err();
        assert!(error.message.contains("cannot be applied"));
    }

    #[test]
    fn propagates_inferred_variable_types() {
        assert!(analyze(&statements(
            "let text = \"safe\"; let result = text + \" code\";"
        ))
        .is_ok());
        assert!(analyze(&statements("let text = \"safe\"; let result = text * 2;")).is_err());
    }

    #[test]
    fn compile_runs_type_analysis_before_execution() {
        let error = compile("print(1 + \"wrong\");").unwrap_err();
        assert_eq!(error.stage, DiagnosticStage::Semantic);
    }
}
