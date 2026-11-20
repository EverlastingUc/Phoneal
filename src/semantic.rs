use crate::{
    ast::{Expression, Statement},
    diagnostic::{Diagnostic, DiagnosticStage, SourceLocation},
    token::TokenKind,
    types::PhonealType,
};
use std::collections::{HashMap, HashSet};
#[derive(Clone)]
struct FunctionSignature {
    parameters: Vec<PhonealType>,
    return_type: PhonealType,
}
type FunctionTable = HashMap<String, FunctionSignature>;
#[derive(Clone, Copy)]
struct BindingInfo {
    ty: Option<PhonealType>,
    mutable: bool,
}
pub fn analyze(statements: &[Statement]) -> Result<(), Diagnostic> {
    analyze_all(statements).into_result()
}
pub fn analyze_all(statements: &[Statement]) -> DiagnosticSet {
    let mut a = Analyzer {
        scopes: vec![HashMap::new()],
        functions: collect_functions(statements),
        seen_functions: HashSet::new(),
        diagnostics: Vec::new(),
        return_types: Vec::new(),
        loop_depth: 0,
        function_scope_depths: Vec::new(),
    };
    a.analyze_statements(statements);
    DiagnosticSet {
        diagnostics: a.diagnostics,
    }
}
fn collect_functions(ss: &[Statement]) -> FunctionTable {
    let mut f = HashMap::new();
    for s in ss {
        if let Statement::Function {
            name,
            parameters,
            return_type,
            ..
        } = s
        {
            f.entry(name.clone()).or_insert_with(|| FunctionSignature {
                parameters: parameters.iter().map(|(_, t, _)| *t).collect(),
                return_type: *return_type,
            });
        }
    }
    f
}
struct Analyzer {
    scopes: Vec<HashMap<String, BindingInfo>>,
    functions: FunctionTable,
    seen_functions: HashSet<String>,
    diagnostics: Vec<Diagnostic>,
    return_types: Vec<PhonealType>,
    loop_depth: usize,
    function_scope_depths: Vec<usize>,
}
impl Analyzer {
    fn report(&mut self, msg: String, loc: &SourceLocation) {
        self.diagnostics.push(Diagnostic::at(
            DiagnosticStage::Semantic,
            msg,
            loc.line,
            loc.column,
        ));
    }
    fn analyze_statements(&mut self, ss: &[Statement]) {
        for s in ss {
            match s {
                Statement::Let {
                    name,
                    mutable,
                    name_location,
                    declared_type,
                    value,
                } => {
                    let dup = self.scopes.last().unwrap().contains_key(name);
                    if dup {
                        self.report(
                            format!("Variable '{name}' is already declared in this scope"),
                            name_location,
                        );
                    }
                    let inferred = self.infer_expression(value);
                    if let (Some(expected), Some(actual)) = (declared_type, inferred) {
                        if *expected != actual {
                            self.report(format!("Variable '{name}' is declared as {expected}, but its value has type {actual}"), name_location);
                        }
                    }
                    let ty = declared_type.map(Some).unwrap_or(inferred);
                    if !dup {
                        self.scopes.last_mut().unwrap().insert(
                            name.clone(),
                            BindingInfo {
                                ty,
                                mutable: *mutable,
                            },
                        );
                    }
                }
                Statement::Assign {
                    name,
                    location,
                    value,
                } => {
                    let actual = self.infer_expression(value);
                    match self.lookup_with_scope(name) {
                        None => self.report(
                            format!("Cannot assign to undefined variable '{name}'"),
                            location,
                        ),
                        Some((scope_index, binding)) => {
                            if !binding.mutable {
                                self.report(
                                    format!("Cannot assign to immutable variable '{name}'"),
                                    location,
                                );
                            } else if self
                                .function_scope_depths
                                .last()
                                .is_some_and(|function_scope| scope_index < *function_scope)
                            {
                                self.report(
                                    format!(
                                        "Functions cannot assign to outer-scope variable '{name}'"
                                    ),
                                    location,
                                );
                            }
                            if let (Some(expected), Some(actual)) = (binding.ty, actual) {
                                if expected != actual {
                                    self.report(
                                        format!("Assignment to '{name}' expects {expected}, received {actual}"),
                                        location,
                                    );
                                }
                            }
                        }
                    }
                }
                Statement::Print(e) => {
                    self.infer_expression(e);
                }
                Statement::Return { value, location } => {
                    let got = self.infer_expression(value);
                    if let Some(expected) = self.return_types.last().copied() {
                        if let Some(actual) = got {
                            if actual != expected {
                                self.report(format!("Return type mismatch: expected {expected}, received {actual}"),location);
                            }
                        }
                    } else {
                        self.report("Return is only valid inside a function".into(), location);
                    }
                }
                Statement::Function {
                    name,
                    name_location,
                    parameters,
                    return_type,
                    return_type_location,
                    body,
                } => {
                    if self.scopes.len() != 1 {
                        self.report(
                            "Function declarations must be at top level".into(),
                            name_location,
                        );
                    }
                    if !self.seen_functions.insert(name.clone()) {
                        self.report(
                            format!("Function '{name}' is already declared"),
                            name_location,
                        );
                    }
                    for (i, (p, _, loc)) in parameters.iter().enumerate() {
                        if parameters[..i].iter().any(|(prev, _, _)| prev == p) {
                            self.report(format!("Duplicate parameter '{p}'"), loc);
                        }
                    }
                    self.return_types.push(*return_type);
                    let enclosing_loop_depth = std::mem::replace(&mut self.loop_depth, 0);
                    self.scopes.push(HashMap::new());
                    self.function_scope_depths.push(self.scopes.len() - 1);
                    for (p, t, _) in parameters {
                        self.scopes.last_mut().unwrap().insert(
                            p.clone(),
                            BindingInfo {
                                ty: Some(*t),
                                mutable: false,
                            },
                        );
                    }
                    self.analyze_statements(body);
                    self.scopes.pop();
                    self.function_scope_depths.pop();
                    self.return_types.pop();
                    self.loop_depth = enclosing_loop_depth;
                    if !guarantees_return(body) {
                        self.report(format!("Function '{name}' does not return a {return_type} value on every path"),return_type_location);
                    }
                }
                Statement::Block { statements, .. } => {
                    self.scopes.push(HashMap::new());
                    self.analyze_statements(statements);
                    self.scopes.pop();
                }
                Statement::While {
                    condition,
                    body,
                    location,
                } => {
                    if let Some(t) = self.infer_expression(condition) {
                        if t != PhonealType::Boolean {
                            self.report(
                                format!("While condition must be boolean, received {t}"),
                                location,
                            );
                        }
                    }
                    self.loop_depth += 1;
                    self.scopes.push(HashMap::new());
                    self.analyze_statements(body);
                    self.scopes.pop();
                    self.loop_depth -= 1;
                }
                Statement::Break { location } => {
                    if self.loop_depth == 0 {
                        self.report("'break' is only valid inside a loop".into(), location);
                    }
                }
                Statement::Continue { location } => {
                    if self.loop_depth == 0 {
                        self.report("'continue' is only valid inside a loop".into(), location);
                    }
                }
                Statement::If {
                    condition,
                    then_branch,
                    else_branch,
                    location,
                } => {
                    if let Some(t) = self.infer_expression(condition) {
                        if t != PhonealType::Boolean {
                            self.report(
                                format!("If condition must be boolean, received {t}"),
                                location,
                            );
                        }
                    }
                    self.scopes.push(HashMap::new());
                    self.analyze_statements(then_branch);
                    self.scopes.pop();
                    if let Some(branch) = else_branch {
                        self.scopes.push(HashMap::new());
                        self.analyze_statements(branch);
                        self.scopes.pop();
                    }
                }
            }
        }
    }
    fn lookup(&self, n: &str) -> Option<BindingInfo> {
        self.lookup_with_scope(n).map(|(_, binding)| binding)
    }
    fn lookup_with_scope(&self, name: &str) -> Option<(usize, BindingInfo)> {
        self.scopes
            .iter()
            .enumerate()
            .rev()
            .find_map(|(index, scope)| scope.get(name).copied().map(|binding| (index, binding)))
    }
    fn infer_expression(&mut self, e: &Expression) -> Option<PhonealType> {
        match e {
            Expression::Integer(_) => Some(PhonealType::Integer),
            Expression::String(_) => Some(PhonealType::Text),
            Expression::Boolean(_) => Some(PhonealType::Boolean),
            Expression::Variable { name, location } => match self.lookup(name) {
                Some(binding) => binding.ty,
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
                let actual = arguments
                    .iter()
                    .map(|e| self.infer_expression(e))
                    .collect::<Vec<_>>();
                match self.functions.get(name).cloned() {
                    Some(sig) => {
                        if sig.parameters.len() != arguments.len() {
                            self.report(
                                format!(
                                    "Function '{name}' expects {} argument(s), received {}",
                                    sig.parameters.len(),
                                    arguments.len()
                                ),
                                location,
                            );
                        } else {
                            for (i, (a, expected)) in actual.iter().zip(&sig.parameters).enumerate()
                            {
                                if let Some(a) = a {
                                    if a != expected {
                                        self.report(format!("Argument {} of '{name}' expects {expected}, received {a}",i+1),location);
                                    }
                                }
                            }
                        }
                        Some(sig.return_type)
                    }
                    None => {
                        self.report(format!("Undefined function '{name}'"), location);
                        None
                    }
                }
            }
            Expression::Group(inner) => self.infer_expression(inner),
            Expression::Unary {
                operator,
                operator_location,
                operand,
            } => {
                let t = self.infer_expression(operand)?;
                let result = match (operator, t) {
                    (TokenKind::Not, PhonealType::Boolean) => Some(PhonealType::Boolean),
                    (TokenKind::Minus, PhonealType::Integer) => Some(PhonealType::Integer),
                    _ => None,
                };
                if result.is_none() {
                    self.report(
                        format!("Unary operator cannot be applied to {t}"),
                        operator_location,
                    );
                }
                result
            }
            Expression::Binary {
                left,
                operator,
                operator_location,
                right,
            } => {
                let lt = self.infer_expression(left);
                let rt = self.infer_expression(right);
                let (Some(l), Some(r)) = (lt, rt) else {
                    return None;
                };
                let result = match (operator, l, r) {
                    (TokenKind::Plus, PhonealType::Integer, PhonealType::Integer) => {
                        Some(PhonealType::Integer)
                    }
                    (TokenKind::Plus, PhonealType::Text, PhonealType::Text) => {
                        Some(PhonealType::Text)
                    }
                    (
                        TokenKind::Minus | TokenKind::Star | TokenKind::Slash | TokenKind::Percent,
                        PhonealType::Integer,
                        PhonealType::Integer,
                    ) => Some(PhonealType::Integer),
                    (
                        TokenKind::Less
                        | TokenKind::LessEqual
                        | TokenKind::Greater
                        | TokenKind::GreaterEqual,
                        PhonealType::Integer,
                        PhonealType::Integer,
                    ) => Some(PhonealType::Boolean),
                    (TokenKind::EqualEqual | TokenKind::BangEqual, a, b) if a == b => {
                        Some(PhonealType::Boolean)
                    }
                    (
                        TokenKind::And | TokenKind::Or,
                        PhonealType::Boolean,
                        PhonealType::Boolean,
                    ) => Some(PhonealType::Boolean),
                    _ => None,
                };
                if result.is_none() {
                    let op = match operator {
                        TokenKind::Plus => "+",
                        TokenKind::Minus => "-",
                        TokenKind::Star => "*",
                        TokenKind::Slash => "/",
                        TokenKind::Percent => "%",
                        TokenKind::Less => "<",
                        TokenKind::LessEqual => "<=",
                        TokenKind::Greater => ">",
                        TokenKind::GreaterEqual => ">=",
                        TokenKind::EqualEqual => "==",
                        TokenKind::BangEqual => "!=",
                        TokenKind::And => "and",
                        TokenKind::Or => "or",
                        _ => "?",
                    };
                    self.report(
                        format!("Operator '{op}' cannot be applied to {l} and {r}"),
                        operator_location,
                    );
                }
                result
            }
        }
    }
}
fn guarantees_return(ss: &[Statement]) -> bool {
    ss.iter().any(|s| match s {
        Statement::Return { .. } => true,
        Statement::Block { statements, .. } => guarantees_return(statements),
        Statement::If {
            then_branch,
            else_branch: Some(other),
            ..
        } => guarantees_return(then_branch) && guarantees_return(other),
        _ => false,
    })
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
            Some(d) => Err(d),
            None => Ok(()),
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use crate::{lexer::Lexer, parser::Parser};
    fn parse(s: &str) -> Vec<Statement> {
        Parser::new(Lexer::new(s).scan().unwrap()).parse().unwrap()
    }
    #[test]
    fn accepts_matching_explicit_variable_type() {
        assert!(analyze(&parse(
            "create age: integer = 18; create name: text = \"Ever\"; create ready: boolean = true;"
        ))
        .is_ok());
    }
    #[test]
    fn rejects_mismatched_explicit_variable_type() {
        assert!(analyze_all(&parse("create age: integer = \"eighteen\";"))
            .iter()
            .any(|d| d.message.contains("declared as integer")));
    }
    #[test]
    fn accepts_valid_mutation() {
        assert!(analyze(&parse("create mutable n: integer = 0; set n = n + 1;")).is_ok());
    }
    #[test]
    fn rejects_immutable_mutation() {
        assert!(analyze_all(&parse("create n = 0; set n = 1;"))
            .iter()
            .any(|d| d.message.contains("immutable variable")));
    }
    #[test]
    fn rejects_mutation_type_mismatch() {
        assert!(
            analyze_all(&parse("create mutable n: integer = 0; set n = \"bad\";"))
                .iter()
                .any(|d| d.message.contains("expects integer"))
        );
    }
    #[test]
    fn rejects_unknown_assignment_target() {
        assert!(analyze_all(&parse("set missing = 1;"))
            .iter()
            .any(|d| d.message.contains("undefined variable")));
    }
    #[test]
    fn rejects_function_mutation_of_outer_bindings() {
        let source = "create mutable total = 0; function update() returns integer { set total = 1; return total; }";
        assert!(analyze_all(&parse(source))
            .iter()
            .any(|d| d.message.contains("cannot assign to outer-scope variable")));
    }
    #[test]
    fn accepts_boolean_condition_and_comparison() {
        assert!(analyze(&parse(
            "create age = 20; if (age >= 18 and true) { print(age); }"
        ))
        .is_ok());
    }
    #[test]
    fn rejects_non_boolean_if_condition() {
        assert!(analyze_all(&parse("if (10) { print(1); }"))
            .iter()
            .any(|d| d.message.contains("must be boolean")));
    }
    #[test]
    fn validates_loop_control_context_and_conditions() {
        assert!(analyze(&parse("while (true) { break; continue; }")).is_ok());
        assert!(analyze_all(&parse("break;"))
            .iter()
            .any(|d| d.message.contains("only valid inside a loop")));
        assert!(analyze_all(&parse("continue;"))
            .iter()
            .any(|d| d.message.contains("only valid inside a loop")));
        assert!(analyze_all(&parse("while (1) {}"))
            .iter()
            .any(|d| d.message.contains("must be boolean")));
    }
    #[test]
    fn checks_function_branch_return_coverage() {
        assert!(analyze(&parse("function choose(flag: boolean) returns integer { if (flag) { return 1; } else { return 2; } }")).is_ok());
    }
    #[test]
    fn rejects_missing_else_return_path() {
        assert!(analyze_all(&parse(
            "function choose(flag: boolean) returns integer { if (flag) { return 1; } }"
        ))
        .iter()
        .any(|d| d.message.contains("does not return")));
    }
    #[test]
    fn rejects_mixed_logical_types() {
        assert!(analyze_all(&parse("create bad = true and 1;"))
            .iter()
            .any(|d| d.message.contains("cannot be applied")));
    }
    #[test]
    fn rejects_return_outside_function() {
        assert!(analyze_all(&parse("return 1;"))
            .iter()
            .any(|d| d.message.contains("only valid inside")));
    }
}
