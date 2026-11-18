use std::collections::HashSet;

use crate::{
    ast::{Expression, Statement},
    diagnostic::{Diagnostic, DiagnosticStage},
};

/// Validates declaration and variable-use rules before interpretation.
///
/// The current language has one program-wide scope. A declaration becomes
/// visible only after its initializer has been validated.
pub fn analyze(statements: &[Statement]) -> Result<(), Diagnostic> {
    let mut declared = HashSet::new();

    for statement in statements {
        match statement {
            Statement::Let {
                name,
                name_location,
                value,
            } => {
                if declared.contains(name) {
                    return Err(Diagnostic::at(
                        DiagnosticStage::Semantic,
                        format!("Variable '{name}' is already declared"),
                        name_location.line,
                        name_location.column,
                    ));
                }

                validate_expression(value, &declared)?;
                declared.insert(name.clone());
            }
            Statement::Print(expression) => {
                validate_expression(expression, &declared)?;
            }
        }
    }

    Ok(())
}

fn validate_expression(
    expression: &Expression,
    declared: &HashSet<String>,
) -> Result<(), Diagnostic> {
    match expression {
        Expression::Integer(_) | Expression::String(_) => Ok(()),
        Expression::Variable { name, location } => {
            if declared.contains(name) {
                Ok(())
            } else {
                Err(Diagnostic::at(
                    DiagnosticStage::Semantic,
                    format!("Undefined variable '{name}'"),
                    location.line,
                    location.column,
                ))
            }
        }
        Expression::Group(inner) => validate_expression(inner, declared),
        Expression::Binary { left, right, .. } => {
            validate_expression(left, declared)?;
            validate_expression(right, declared)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{compile, diagnostic::DiagnosticStage};

    fn statements(source: &str) -> Vec<Statement> {
        let tokens = crate::lexer::Lexer::new(source).scan().unwrap();
        crate::parser::Parser::new(tokens).parse().unwrap()
    }

    #[test]
    fn accepts_declared_variables() {
        assert!(analyze(&statements(
            "let count = 3; let total = count + 4; print(total);"
        ))
        .is_ok());
    }

    #[test]
    fn rejects_undefined_variable_in_print_with_exact_location() {
        let error = analyze(&statements("print(missing);")).unwrap_err();

        assert_eq!(error.stage, DiagnosticStage::Semantic);
        assert_eq!(error.message, "Undefined variable 'missing'");
        let location = error.location.unwrap();
        assert_eq!(location.line, 1);
        assert_eq!(location.column, 7);
    }

    #[test]
    fn rejects_undefined_variable_in_initializer_with_exact_location() {
        let error = analyze(&statements("let result = unknown + 1;")).unwrap_err();

        assert_eq!(error.message, "Undefined variable 'unknown'");
        let location = error.location.unwrap();
        assert_eq!(location.line, 1);
        assert_eq!(location.column, 14);
    }

    #[test]
    fn rejects_duplicate_declarations_at_second_name() {
        let error = analyze(&statements("let value = 1; let value = 2;")).unwrap_err();

        assert_eq!(error.message, "Variable 'value' is already declared");
        let location = error.location.unwrap();
        assert_eq!(location.line, 1);
        assert_eq!(location.column, 20);
    }

    #[test]
    fn declaration_is_not_visible_inside_its_own_initializer() {
        let error = analyze(&statements("let value = value + 1;")).unwrap_err();

        assert_eq!(error.message, "Undefined variable 'value'");
        assert_eq!(error.location.unwrap().column, 13);
    }

    #[test]
    fn compile_runs_semantic_analysis_before_execution() {
        let error = compile("print(not_declared);").unwrap_err();

        assert_eq!(error.stage, DiagnosticStage::Semantic);
        assert!(error.message.contains("not_declared"));
        assert_eq!(error.location.unwrap().column, 7);
    }

    #[test]
    fn detects_undefined_variables_inside_grouped_expressions() {
        let error = analyze(&statements("print((known + missing));")).unwrap_err();

        assert_eq!(error.message, "Undefined variable 'known'");
        assert_eq!(error.location.unwrap().column, 8);
    }
}
