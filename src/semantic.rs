use std::collections::HashSet;

use crate::{
    ast::{Expression, Statement},
    diagnostic::{Diagnostic, DiagnosticStage},
};

/// Validates declaration and variable-use rules before interpretation.
///
/// The current language has one program-wide scope. A declaration becomes
/// visible only after its initializer has been checked.
pub fn analyze(statements: &[Statement]) -> Result<(), Diagnostic> {
    analyze_all(statements).into_result()
}

/// Runs semantic analysis and returns every independent diagnostic found.
pub fn analyze_all(statements: &[Statement]) -> DiagnosticSet {
    let mut declared = HashSet::new();
    let mut diagnostics = Vec::new();

    for statement in statements {
        match statement {
            Statement::Let {
                name,
                name_location,
                value,
            } => {
                if declared.contains(name) {
                    diagnostics.push(Diagnostic::at(
                        DiagnosticStage::Semantic,
                        format!("Variable '{name}' is already declared"),
                        name_location.line,
                        name_location.column,
                    ));
                }

                validate_expression(value, &declared, &mut diagnostics);

                // Keep the first declaration authoritative. A duplicate must
                // never replace the symbol already present in this scope.
                declared.insert(name.clone());
            }
            Statement::Print(expression) => {
                validate_expression(expression, &declared, &mut diagnostics);
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

fn validate_expression(
    expression: &Expression,
    declared: &HashSet<String>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    match expression {
        Expression::Integer(_) | Expression::String(_) => {}
        Expression::Variable { name, location } => {
            if !declared.contains(name) {
                diagnostics.push(Diagnostic::at(
                    DiagnosticStage::Semantic,
                    format!("Undefined variable '{name}'"),
                    location.line,
                    location.column,
                ));
            }
        }
        Expression::Group(inner) => validate_expression(inner, declared, diagnostics),
        Expression::Binary { left, right, .. } => {
            validate_expression(left, declared, diagnostics);
            validate_expression(right, declared, diagnostics);
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
    fn duplicate_declaration_does_not_hide_initializer_errors() {
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
}
