use crate::lexer::{ Lexer, Token };
use super::node::ParseError;

pub fn not_reserved(s: &str, lexer: &mut Lexer) -> Result<(),Vec<ParseError>> {
    if s == "reserved" {
        Err(vec![ParseError::new(&format!("Reserved keyword '{}' found",s),lexer)])?;
    }
    Ok(())
}

pub fn get_string(lexer: &mut Lexer) -> Result<String,Vec<ParseError>> {
    match lexer.get() {
        Token::LiteralString(symbol) => Ok(symbol),
        _ => Err(vec![ParseError::new("expected string",lexer)])
    }
}

pub fn get_number(lexer: &mut Lexer) -> Result<f64,Vec<ParseError>> {
    match lexer.get() {
        Token::Number(number) => Ok(number),
        _ => Err(vec![ParseError::new("expected number",lexer)])
    }
}

pub fn get_identifier(lexer: &mut Lexer) -> Result<String,Vec<ParseError>> {
    match lexer.get() {
        Token::Identifier(symbol) => Ok(symbol),
        _ => Err(vec![ParseError::new("expected identifier",lexer)])
    }
}

pub fn get_operator(lexer: &mut Lexer) -> Result<String,Vec<ParseError>> {
    match lexer.get() {
        Token::Operator(symbol) => Ok(symbol),
        _ => Err(vec![ParseError::new("expected operator",lexer)])
    }
}

pub fn get_other(lexer: &mut Lexer, ok: &str) -> Result<char,Vec<ParseError>> {
    let out = match lexer.get() {
        Token::Other(symbol) => Ok(symbol),
        _ => Err(vec![ParseError::new(&format!("Expected one of \"{}\"",ok),lexer)])
    }?;
    if !ok.contains(out) {
        Err(vec![ParseError::new(&format!("Expected one of \"{}\"",ok),lexer)])?;
    }
    Ok(out)
}
