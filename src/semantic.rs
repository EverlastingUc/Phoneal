use std::collections::HashMap;

use crate::{
    ast::{Expression, Statement},
    diagnostic::{Diagnostic, DiagnosticStage},
    token::TokenKind,
    types::PhonealType,
};

/// Validates names and expression types using nested lexical scopes.
pub fn analyze(statements: &[Statement]) -> Result<(), Diagnostic> {
    analyze_all(statements).into_result()
}

/// Runs semantic analysis and returns every independent diagnostic found.
pub fn analyze_all(statements: &[Statement]) -> DiagnosticSet {
    let mut analyzer = Analyzer {
        scopes: vec![HashMap::new()],
        diagnostics: Vec::new(),
    };
    analyzer.analyze_statements(statements);
    DiagnosticSet {
        diagnostics: analyzer.diagnostics,
    }
}

struct Analyzer {
    scopes: Vec<HashMap<String, Option<PhonealType>>>,
    diagnostics: Vec<Diagnostic>,
}

impl Analyzer {
    fn analyze_statements(&mut self, statements: &[Statement]) {
        for statement in statements {
            match statement {
                Statement::Let {
                    name,
                    name_location,
                    value,
                } => {
                    let current_scope = self.scopes.last().expect("scope stack is never empty");
                    let already_declared = current_scope.contains_key(name);
                    if already_declared {
                        self.diagnostics.push(Diagnostic::at(
                            DiagnosticStage::Semantic,
                            format!("Variable '{name}' is already declared in this scope"),
                            name_location.line,
                            name_location.column,
                        ));
                    }

                    let inferred = self.infer_expression(value);
                    if !already_declared {
                        self.scopes
                            .last_mut()
                            .expect("scope stack is never empty")
                            .insert(name.clone(), inferred);
                    }
                }
                Statement::Print(expression) => {
                    self.infer_expression(expression);
                }
                Statement::Block { statements, .. } => {
                    self.scopes.push(HashMap::new());
                    self.analyze_statements(statements);
                    self.scopes.pop();
                }
            }
        }
    }

    fn lookup(&self, name: &str) -> Option<Option<PhonealType>> {
        self.scopes
            .iter()
            .rev()
            .find_map(|scope| scope.get(name).copied())
    }

    fn infer_expression(&mut self, expression: &Expression) -> Option<PhonealType> {
        match expression {
            Expression::Integer(_) => Some(PhonealType::Integer),
            Expression::String(_) => Some(PhonealType::String),
            Expression::Variable { name, location } => match self.lookup(name) {
                Some(value_type) => value_type,
                None => {
                    self.diagnostics.push(Diagnostic::at(
                        DiagnosticStage::Semantic,
                        format!("Undefined variable '{name}'"),
                        location.line,
                        location.column,
                    ));
                    None
                }
            },
            Expression::Group(inner) => self.infer_expression(inner),
            Expression::Binary {
                left,
                operator,
                operator_location,
                right,
            } => {
                let left_type = self.infer_expression(left);
                let right_type = self.infer_expression(right);
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
                    self.diagnostics.push(Diagnostic::at(
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
        assert_eq!(
            error.message,
            "Variable 'value' is already declared in this scope"
        );
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
            "Variable 'value' is already declared in this scope"
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
        assert_eq!(error.location.unwrap().column, 16);
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

    #[test]
    fn allows_shadowing_in_nested_scope() {
        assert!(analyze(&statements(
            "let value = 1; { let value = \"inner\"; print(value); } print(value);"
        ))
        .is_ok());
    }

    #[test]
    fn rejects_duplicate_names_inside_same_block() {
        let error = analyze(&statements("{ let item = 1; let item = 2; }")).unwrap_err();
        assert!(error.message.contains("already declared in this scope"));
    }

    #[test]
    fn block_local_name_does_not_escape_scope() {
        let result = analyze_all(&statements("{ let hidden = 1; } print(hidden);"));
        assert_eq!(result.len(), 1);
        assert_eq!(result.iter().next().unwrap().message, "Undefined variable 'hidden'");
    }

    #[test]
    fn nested_blocks_resolve_outer_variables() {
        assert!(analyze(&statements(
            "let outer = 1; { { print(outer); } }"
        ))
        .is_ok());
    }
}
