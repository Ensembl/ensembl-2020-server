use crate::lexer::{ Lexer, Token };
use super::node::ParseError;

pub fn not_reserved(s: &str, lexer: &mut Lexer) -> Result<(),ParseError> {
    if s == "reserved" {
        Err(ParseError::new(&format!("Reserved keyword '{}' found",s),lexer))?;
    }
    Ok(())
}

pub fn get_string(lexer: &mut Lexer) -> Result<String,ParseError> {
    match lexer.get() {
        Token::LiteralString(symbol) => Ok(symbol),
        _ => Err(ParseError::new("expected string",lexer))
    }
}

pub fn get_number(lexer: &mut Lexer) -> Result<f64,ParseError> {
    match lexer.get() {
        Token::Number(number,_) => Ok(number),
        _ => Err(ParseError::new("expected number",lexer))
    }
}

pub fn get_identifier(lexer: &mut Lexer) -> Result<String,ParseError> {
    match lexer.get() {
        Token::Identifier(symbol) => Ok(symbol),
        Token::Number(_,symbol) => Ok(symbol),
        x => Err(ParseError::new(&format!("expected identifier, got {:?}",x),lexer))
    }
}

pub fn get_operator(lexer: &mut Lexer) -> Result<String,ParseError> {
    match lexer.get() {
        Token::Operator(symbol) => Ok(symbol),
        _ => Err(ParseError::new("expected operator",lexer))
    }
}

pub fn get_other(lexer: &mut Lexer, ok: &str) -> Result<char,ParseError> {
    let out = match lexer.get() {
        Token::Other(symbol) => Ok(symbol),
        _ => Err(ParseError::new(&format!("Expected one of \"{}\"",ok),lexer))
    }?;
    if !ok.contains(out) {
        Err(ParseError::new(&format!("Expected one of \"{}\"",ok),lexer))?;
    }
    Ok(out)
}
