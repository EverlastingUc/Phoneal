use std::collections::{HashMap, HashSet};

use crate::{
    ast::{Expression, Statement},
    diagnostic::{Diagnostic, DiagnosticStage},
    token::TokenKind,
    types::PhonealType,
};

type FunctionTable = HashMap<String, usize>;

pub fn analyze(statements: &[Statement]) -> Result<(), Diagnostic> {
    analyze_all(statements).into_result()
}

pub fn analyze_all(statements: &[Statement]) -> DiagnosticSet {
    let mut analyzer = Analyzer {
        scopes: vec![HashMap::new()],
        functions: collect_functions(statements),
        seen_functions: HashSet::new(),
        diagnostics: Vec::new(),
        in_function: false,
    };
    analyzer.analyze_statements(statements);
    DiagnosticSet {
        diagnostics: analyzer.diagnostics,
    }
}

fn collect_functions(statements: &[Statement]) -> FunctionTable {
    let mut functions = HashMap::new();
    for statement in statements {
        if let Statement::Function {
            name, parameters, ..
        } = statement
        {
            functions.insert(name.clone(), parameters.len());
        }
    }
    functions
}

struct Analyzer {
    scopes: Vec<HashMap<String, Option<PhonealType>>>,
    functions: FunctionTable,
    seen_functions: HashSet<String>,
    diagnostics: Vec<Diagnostic>,
    in_function: bool,
}

impl Analyzer {
    fn report(&mut self, message: String, location: &crate::diagnostic::SourceLocation) {
        self.diagnostics.push(Diagnostic::at(
            DiagnosticStage::Semantic,
            message,
            location.line,
            location.column,
        ));
    }

    fn analyze_statements(&mut self, statements: &[Statement]) {
        for statement in statements {
            match statement {
                Statement::Let {
                    name,
                    name_location,
                    value,
                } => {
                    let duplicate = self
                        .scopes
                        .last()
                        .expect("scope stack is never empty")
                        .contains_key(name);
                    if duplicate {
                        self.report(
                            format!("Variable '{name}' is already declared in this scope"),
                            name_location,
                        );
                    }
                    let inferred = self.infer_expression(value);
                    if !duplicate {
                        self.scopes
                            .last_mut()
                            .expect("scope stack is never empty")
                            .insert(name.clone(), inferred);
                    }
                }
                Statement::Print(expression) => {
                    self.infer_expression(expression);
                }
                Statement::Return { value, location } => {
                    if !self.in_function {
                        self.report(
                            "Return is only valid inside a function".to_owned(),
                            location,
                        );
                    }
                    self.infer_expression(value);
                }
                Statement::Function {
                    name,
                    name_location,
                    parameters,
                    body,
                } => {
                    if self.scopes.len() != 1 {
                        self.report("Function declarations must be at top level".to_owned(), name_location);
                    }
                    if !self.seen_functions.insert(name.clone()) {
                        self.report(format!("Function '{name}' is already declared"), name_location);
                    }
                    if parameters
                        .iter()
                        .enumerate()
                        .any(|(index, (parameter, _))| {
                            parameters[..index]
                                .iter()
                                .any(|(previous, _)| previous == parameter)
                        })
                    {
                        for (index, (parameter, location)) in parameters.iter().enumerate() {
                            if parameters[..index]
                                .iter()
                                .any(|(previous, _)| previous == parameter)
                            {
                                self.report(format!("Duplicate parameter '{parameter}'"), location);
                            }
                        }
                    }
                    let previous = self.in_function;
                    self.in_function = true;
                    self.scopes.push(HashMap::new());
                    for (parameter, _) in parameters {
                        self.scopes
                            .last_mut()
                            .expect("scope stack is never empty")
                            .insert(parameter.clone(), None);
                    }
                    self.analyze_statements(body);
                    self.scopes.pop();
                    self.in_function = previous;
                    let _ = (name, name_location);
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
                    self.report(format!("Undefined variable '{name}'"), location);
                    None
                }
            },
            Expression::Call {
                name,
                location,
                arguments,
            } => {
                for argument in arguments {
                    self.infer_expression(argument);
                }
                match self.functions.get(name).copied() {
                    Some(expected) if expected == arguments.len() => None,
                    Some(expected) => {
                        self.report(
                            format!(
                                "Function '{name}' expects {expected} argument(s), received {}",
                                arguments.len()
                            ),
                            location,
                        );
                        None
                    }
                    None => {
                        self.report(format!("Undefined function '{name}'"), location);
                        None
                    }
                }
            }
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
                    self.report(
                        format!(
                            "Operator '{symbol}' cannot be applied to {left_type} and {right_type}"
                        ),
                        operator_location,
                    );
                }
                result
            }
        }
    }
}

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
    use crate::{lexer::Lexer, parser::Parser};

    fn statements(source: &str) -> Vec<Statement> {
        Parser::new(Lexer::new(source).scan().unwrap())
            .parse()
            .unwrap()
    }

    #[test]
    fn accepts_function_declarations_calls_and_returns() {
        let source = "fn add(a, b) { return a + b; } let answer = add(1, 2); print(answer);";
        assert!(analyze(&statements(source)).is_ok());
    }

    #[test]
    fn rejects_undefined_function() {
        let errors = analyze_all(&statements("print(missing(1));"));
        assert_eq!(errors.len(), 1);
        assert!(errors
            .iter()
            .next()
            .unwrap()
            .message
            .contains("Undefined function"));
    }

    #[test]
    fn rejects_wrong_function_arity() {
        let errors = analyze_all(&statements("fn add(a, b) { return a + b; } print(add(1));"));
        assert!(errors
            .iter()
            .any(|error| error.message.contains("expects 2 argument")));
    }

    #[test]
    fn rejects_return_outside_function() {
        let errors = analyze_all(&statements("return 1;"));
        assert!(errors
            .iter()
            .any(|error| error.message.contains("only valid inside")));
    }

    #[test]
    fn rejects_duplicate_parameters() {
        let errors = analyze_all(&statements("fn bad(x, x) { return x; }"));
        assert!(errors
            .iter()
            .any(|error| error.message.contains("Duplicate parameter")));
    }

    #[test]
    fn reports_undefined_variable_with_location() {
        let error = analyze(&statements("print(missing);")).unwrap_err();
        assert_eq!(error.location.unwrap().column, 7);
    }

    #[test]
    fn accepts_nested_lexical_scopes() {
        assert!(analyze(&statements(
            "let value = 1; { let value = 2; print(value); }"
        ))
        .is_ok());
    }
}
