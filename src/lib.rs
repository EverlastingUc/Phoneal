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
use semantic::analyze;

pub fn compile(source: &str) -> Result<Vec<Statement>, Diagnostic> {
    let tokens = Lexer::new(source).scan()?;
    let statements = Parser::new(tokens).parse()?;
    analyze(&statements)?;
    Ok(statements)
}

pub fn run(source: &str) -> Result<Vec<String>, Diagnostic> {
    let statements = compile(source)?;
    execute(&statements).map_err(Diagnostic::runtime)
}
