pub mod ast;
pub mod diagnostic;
pub mod interpreter;
pub mod lexer;
pub mod parser;
pub mod semantic;
pub mod token;

use ast::Statement;
use diagnostic::Diagnostic;
use interpreter::execute;
use lexer::Lexer;
use parser::Parser;
use semantic::analyze_all;

pub fn compile_with_diagnostics(
    source: &str,
) -> Result<Vec<Statement>, Vec<Diagnostic>> {
    let tokens = Lexer::new(source)
        .scan()
        .map_err(|diagnostic| vec![diagnostic])?;
    let statements = Parser::new(tokens)
        .parse()
        .map_err(|diagnostic| vec![diagnostic])?;
    let diagnostics = analyze_all(&statements).into_vec();

    if diagnostics.is_empty() {
        Ok(statements)
    } else {
        Err(diagnostics)
    }
}

/// Compiles source and returns the first diagnostic for compatibility.
pub fn compile(source: &str) -> Result<Vec<Statement>, Diagnostic> {
    compile_with_diagnostics(source).map_err(|mut diagnostics| diagnostics.remove(0))
}

pub fn run(source: &str) -> Result<Vec<String>, Diagnostic> {
    let statements = compile(source)?;
    execute(&statements)
}
