use super::node::Statement;
use crate::lexer::Lexer;

fn run_import(path: &str, lexer: &mut Lexer) -> Result<(),String> {
    lexer.import(path)
}

pub fn preprocess(stmt: &Statement, lexer: &mut Lexer) -> Result<bool,String> {
    match stmt {
        Statement::Import(path) => run_import(path,lexer).map(|_| true),
        _ => { return Ok(false); }
    }
}