/* 
 *  See the NOTICE file distributed with this work for additional information
 *  regarding copyright ownership.
 *  
 *  Licensed under the Apache License, Version 2.0 (the "License"); you may 
 *  not use this file except in compliance with the License. You may obtain a
 *  copy of the License at http://www.apache.org/licenses/LICENSE-2.0
 *  
 *  Unless required by applicable law or agreed to in writing, software
 *  distributed under the License is distributed on an "AS IS" BASIS, WITHOUT 
 *  WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 *  
 *  See the License for the specific language governing permissions and
 *  limitations under the License.
 */

use crate::lexer::{ Lexer, Token };
use super::node::ParseError;

lazy_static! {
    static ref KEYWORDS: Vec<&'static str> = {
        vec!["reserved","struct","enum","func","proc","expr","stmt","inline","import","becomes","lvalue","out"]
    };
}

pub fn not_reserved(s: &str, lexer: &mut Lexer) -> Result<(),ParseError> {
    if KEYWORDS.contains(&s) {
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

pub fn get_operator(lexer: &mut Lexer, mode: bool) -> Result<String,ParseError> {
    match lexer.get_oper(mode) {
        Token::Operator(symbol) => Ok(symbol),
        x => Err(ParseError::new(&format!("expected operator not {:?}",x),lexer))
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
