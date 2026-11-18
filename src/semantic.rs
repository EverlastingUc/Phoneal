use std::collections::HashSet;

use crate::{
    ast::{Expression, Statement},
    diagnostic::Diagnostic,
};

/// Validates declaration and variable-use rules before interpretation.
///
/// The current language has one program-wide scope. A declaration becomes
/// visible only after its initializer has been validated.
pub fn analyze(statements: &[Statement]) -> Result<(), Diagnostic> {
    let mut declared = HashSet::new();

    for statement in statements {
        match statement {
            Statement::Let { name, value } => {
                if declared.contains(name) {
                    return Err(Diagnostic::semantic(format!(
                        "Variable '{name}' is already declared"
                    )));
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
        Expression::Variable(name) => {
            if declared.contains(name) {
                Ok(())
            } else {
                Err(Diagnostic::semantic(format!(
                    "Undefined variable '{name}'"
                )))
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
    fn rejects_undefined_variable_in_print() {
        let error = analyze(&statements("print(missing);")).unwrap_err();

        assert_eq!(error.stage, DiagnosticStage::Semantic);
        assert_eq!(error.message, "Undefined variable 'missing'");
    }

    #[test]
    fn rejects_undefined_variable_in_initializer() {
        let error = analyze(&statements("let result = unknown + 1;")).unwrap_err();

        assert_eq!(error.message, "Undefined variable 'unknown'");
    }

    #[test]
    fn rejects_duplicate_declarations() {
        let error = analyze(&statements("let value = 1; let value = 2;")).unwrap_err();

        assert_eq!(error.message, "Variable 'value' is already declared");
    }

    #[test]
    fn declaration_is_not_visible_inside_its_own_initializer() {
        let error = analyze(&statements("let value = value + 1;")).unwrap_err();

        assert_eq!(error.message, "Undefined variable 'value'");
    }

    #[test]
    fn compile_runs_semantic_analysis_before_execution() {
        let error = compile("print(not_declared);").unwrap_err();

        assert_eq!(error.stage, DiagnosticStage::Semantic);
        assert!(error.message.contains("not_declared"));
    }

    #[test]
    fn detects_undefined_variables_inside_grouped_expressions() {
        let error = analyze(&statements("print((known + missing));")).unwrap_err();

        assert_eq!(error.message, "Undefined variable 'known'");
    }
}
