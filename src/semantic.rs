use std::collections::{HashMap, HashSet};

use crate::{
    ast::{Expression, Statement},
    diagnostic::{Diagnostic, DiagnosticStage, SourceLocation},
    token::TokenKind,
    types::PhonealType,
};

#[derive(Clone)]
struct FunctionSignature {
    parameters: Vec<PhonealType>,
    return_type: PhonealType,
}
type FunctionTable = HashMap<String, FunctionSignature>;

pub fn analyze(statements: &[Statement]) -> Result<(), Diagnostic> {
    analyze_all(statements).into_result()
}

pub fn analyze_all(statements: &[Statement]) -> DiagnosticSet {
    let mut analyzer = Analyzer {
        scopes: vec![HashMap::new()],
        functions: collect_functions(statements),
        seen_functions: HashSet::new(),
        diagnostics: Vec::new(),
        return_types: Vec::new(),
    };
    analyzer.analyze_statements(statements);
    DiagnosticSet { diagnostics: analyzer.diagnostics }
}

fn collect_functions(statements: &[Statement]) -> FunctionTable {
    let mut functions = HashMap::new();
    for statement in statements {
        if let Statement::Function { name, parameters, return_type, .. } = statement {
            functions.entry(name.clone()).or_insert_with(|| FunctionSignature {
                parameters: parameters.iter().map(|(_, ty, _)| *ty).collect(),
                return_type: *return_type,
            });
        }
    }
    functions
}

struct Analyzer {
    scopes: Vec<HashMap<String, Option<PhonealType>>>,
    functions: FunctionTable,
    seen_functions: HashSet<String>,
    diagnostics: Vec<Diagnostic>,
    return_types: Vec<PhonealType>,
}

impl Analyzer {
    fn report(&mut self, message: String, location: &SourceLocation) {
        self.diagnostics.push(Diagnostic::at(DiagnosticStage::Semantic, message, location.line, location.column));
    }

    fn analyze_statements(&mut self, statements: &[Statement]) {
        for statement in statements {
            match statement {
                Statement::Let { name, name_location, value } => {
                    let duplicate = self.scopes.last().expect("scope stack is never empty").contains_key(name);
                    if duplicate {
                        self.report(format!("Variable '{name}' is already declared in this scope"), name_location);
                    }
                    let inferred = self.infer_expression(value);
                    if !duplicate {
                        self.scopes.last_mut().expect("scope stack is never empty").insert(name.clone(), inferred);
                    }
                }
                Statement::Print(expression) => { self.infer_expression(expression); }
                Statement::Return { value, location } => {
                    let inferred = self.infer_expression(value);
                    if let Some(expected) = self.return_types.last().copied() {
                        if let Some(actual) = inferred {
                            if actual != expected {
                                self.report(format!("Return type mismatch: expected {expected}, received {actual}"), location);
                            }
                        }
                    } else {
                        self.report("Return is only valid inside a function".to_owned(), location);
                    }
                }
                Statement::Function { name, name_location, parameters, return_type, return_type_location, body } => {
                    if self.scopes.len() != 1 {
                        self.report("Function declarations must be at top level".to_owned(), name_location);
                    }
                    if !self.seen_functions.insert(name.clone()) {
                        self.report(format!("Function '{name}' is already declared"), name_location);
                    }
                    for (index, (parameter, _, location)) in parameters.iter().enumerate() {
                        if parameters[..index].iter().any(|(previous, _, _)| previous == parameter) {
                            self.report(format!("Duplicate parameter '{parameter}'"), location);
                        }
                    }
                    self.return_types.push(*return_type);
                    self.scopes.push(HashMap::new());
                    for (parameter, parameter_type, _) in parameters {
                        self.scopes.last_mut().expect("scope stack is never empty").insert(parameter.clone(), Some(*parameter_type));
                    }
                    self.analyze_statements(body);
                    self.scopes.pop();
                    self.return_types.pop();
                    if !guarantees_return(body) {
                        self.report(format!("Function '{name}' does not return a {return_type} value on every path"), return_type_location);
                    }
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
        self.scopes.iter().rev().find_map(|scope| scope.get(name).copied())
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
            Expression::Call { name, location, arguments } => {
                let argument_types = arguments.iter().map(|argument| self.infer_expression(argument)).collect::<Vec<_>>();
                match self.functions.get(name).cloned() {
                    Some(signature) => {
                        if signature.parameters.len() != arguments.len() {
                            self.report(format!("Function '{name}' expects {} argument(s), received {}", signature.parameters.len(), arguments.len()), location);
                        } else {
                            for (index, (actual, expected)) in argument_types.iter().zip(&signature.parameters).enumerate() {
                                if let Some(actual) = actual {
                                    if actual != expected {
                                        self.report(format!("Argument {} of '{name}' expects {expected}, received {actual}", index + 1), location);
                                    }
                                }
                            }
                        }
                        Some(signature.return_type)
                    }
                    None => {
                        self.report(format!("Undefined function '{name}'"), location);
                        None
                    }
                }
            }
            Expression::Group(inner) => self.infer_expression(inner),
            Expression::Binary { left, operator, operator_location, right } => {
                let left_type = self.infer_expression(left);
                let right_type = self.infer_expression(right);
                let (Some(left_type), Some(right_type)) = (left_type, right_type) else { return None; };
                let result = match (operator, left_type, right_type) {
                    (TokenKind::Plus, PhonealType::Integer, PhonealType::Integer) => Some(PhonealType::Integer),
                    (TokenKind::Plus, PhonealType::String, PhonealType::String) => Some(PhonealType::String),
                    (TokenKind::Minus | TokenKind::Star | TokenKind::Slash, PhonealType::Integer, PhonealType::Integer) => Some(PhonealType::Integer),
                    _ => None,
                };
                if result.is_none() {
                    let symbol = match operator { TokenKind::Plus => "+", TokenKind::Minus => "-", TokenKind::Star => "*", TokenKind::Slash => "/", _ => "?" };
                    self.report(format!("Operator '{symbol}' cannot be applied to {left_type} and {right_type}"), operator_location);
                }
                result
            }
        }
    }
}

fn guarantees_return(statements: &[Statement]) -> bool {
    statements.iter().any(|statement| match statement {
        Statement::Return { .. } => true,
        Statement::Block { statements, .. } => guarantees_return(statements),
        _ => false,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticSet { diagnostics: Vec<Diagnostic> }

impl DiagnosticSet {
    pub fn is_empty(&self) -> bool { self.diagnostics.is_empty() }
    pub fn len(&self) -> usize { self.diagnostics.len() }
    pub fn iter(&self) -> impl Iterator<Item = &Diagnostic> { self.diagnostics.iter() }
    pub fn into_vec(self) -> Vec<Diagnostic> { self.diagnostics }
    pub fn into_result(self) -> Result<(), Diagnostic> {
        match self.diagnostics.into_iter().next() { Some(diagnostic) => Err(diagnostic), None => Ok(()) }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{lexer::Lexer, parser::Parser};

    fn statements(source: &str) -> Vec<Statement> {
        Parser::new(Lexer::new(source).scan().unwrap()).parse().unwrap()
    }

    #[test]
    fn accepts_typed_function_calls_and_returns() {
        assert!(analyze(&statements("fn add(a: integer, b: integer) -> integer { return a + b; } print(add(1, 2));")).is_ok());
    }

    #[test]
    fn rejects_argument_type_mismatch() {
        let errors = analyze_all(&statements("fn add(a: integer) -> integer { return a; } print(add(\"wrong\"));"));
        assert!(errors.iter().any(|error| error.message.contains("expects integer, received string")));
    }

    #[test]
    fn rejects_return_type_mismatch() {
        let errors = analyze_all(&statements("fn broken() -> integer { return \"wrong\"; }"));
        assert!(errors.iter().any(|error| error.message.contains("Return type mismatch")));
    }

    #[test]
    fn rejects_functions_without_guaranteed_return() {
        let errors = analyze_all(&statements("fn broken() -> integer { let value = 1; }"));
        assert!(errors.iter().any(|error| error.message.contains("does not return")));
    }

    #[test]
    fn accepts_return_inside_unconditional_block() {
        assert!(analyze(&statements("fn value() -> integer { { return 42; } }")).is_ok());
    }

    #[test]
    fn rejects_return_outside_function() {
        let errors = analyze_all(&statements("return 1;"));
        assert!(errors.iter().any(|error| error.message.contains("only valid inside")));
    }
}
