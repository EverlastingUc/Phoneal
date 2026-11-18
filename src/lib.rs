pub mod ast;
pub mod interpreter;
pub mod lexer;
pub mod parser;
pub mod token;

use ast::Statement;
use interpreter::execute;
use lexer::Lexer;
use parser::Parser;

pub fn compile(source: &str) -> Result<Vec<Statement>, String> {
    let tokens = Lexer::new(source).scan()?;
    Parser::new(tokens).parse()
}

pub fn run(source: &str) -> Result<Vec<String>, String> {
    let statements = compile(source)?;
    execute(&statements)
}
