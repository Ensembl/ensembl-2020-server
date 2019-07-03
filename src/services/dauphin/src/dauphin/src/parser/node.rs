use super::inline::InlineMode;
use crate::lexer::Lexer;

#[derive(Debug,PartialEq)]
pub enum Statement {
    Import(String),
    Inline(String,String,InlineMode,f64),
    Regular(String)
}

#[derive(Debug,PartialEq)]
pub struct ParseError {
    error: String
}

impl ParseError {
    pub fn new(error: &str, lexer: &Lexer) -> ParseError {
        let (file,line,col) = lexer.position();
        ParseError {
            error: format!("{} at line {} column {} in {}",error,line,col,file)
        }
    }

    pub fn message(&self) -> &str { &self.error }
}