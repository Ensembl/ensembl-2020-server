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
use super::node::{ Statement, ParserStatement, ParseError, Expression };
use super::lexutil::{ get_string, get_other, id_not_reserved, get_identifier, get_number, get_operator };
use crate::model::{ DefStore, InlineMode, StmtMacro };
use super::parsedecl::{ 
    parse_proc, parse_exprdecl, parse_stmtdecl, parse_func, parse_struct,
    parse_enum
};
use super::parseexpr::{ parse_expr, parse_exprlist, parse_full_identifier, peek_full_identifier };

fn parse_regular(lexer: &mut Lexer, defstore: &DefStore) -> Result<Vec<ParserStatement>,ParseError> {
    if let Some(pattern) = peek_full_identifier(lexer,None) {
        let identifier = defstore.pattern_to_identifier(lexer,&pattern,true).map_err(|e| ParseError::new(&e.to_string(),lexer))?;
        if defstore.stmt_like(&identifier.0,lexer).unwrap_or(false) {
            return parse_funcstmt(lexer,defstore);
        }
    }
    parse_inlinestmt(lexer,defstore)
}

fn parse_import(lexer: &mut Lexer) -> Result<Vec<ParserStatement>,ParseError> {
    lexer.get();
    Ok(vec![ParserStatement::Import(get_string(lexer)?)])
}

fn parse_use(lexer: &mut Lexer, _defstore: &DefStore) -> Result<Vec<ParserStatement>,ParseError> {
    lexer.get();
    Ok(vec![ParserStatement::Use(get_string(lexer)?)])
}

fn parse_module(lexer: &mut Lexer) -> Result<Vec<ParserStatement>,ParseError> {
    lexer.get();
    Ok(vec![ParserStatement::Module(get_string(lexer)?)])
}

fn parse_inline(lexer: &mut Lexer) -> Result<Vec<ParserStatement>,ParseError> {
    lexer.get();
    let symbol = get_string(lexer)?;
    let pattern = parse_full_identifier(lexer,None)?;
    let mode = match &get_identifier(lexer)?[..] {
        "left" => Ok(InlineMode::LeftAssoc),
        "right" => Ok(InlineMode::RightAssoc),
        "prefix" => Ok(InlineMode::Prefix),
        "suffix" => Ok(InlineMode::Suffix),
        _ => Err(ParseError::new("Bad oper mode, expected left, right, prefix, or suffix",lexer))
    }?;
    let num = get_number(lexer)?;
    if let Some(prio) = num.parse::<f64>().ok() {
        Ok(vec![ParserStatement::Inline(symbol,pattern,mode,prio)])
    } else {
        Err(ParseError::new(&format!("bad priority '{}'",num),lexer))
    }
    
}

fn apply_macro(s: &StmtMacro, exprs: &[Expression], lexer: &mut Lexer)-> Result<Vec<ParserStatement>,ParseError> {
    let exprs = s.block(exprs).map_err(|e| ParseError::new(&e.to_string(),lexer))?;
    Ok(exprs.iter().map(|x| ParserStatement::Regular(x.clone())).collect())
}

fn parse_funcstmt(lexer: &mut Lexer, defstore: &DefStore)-> Result<Vec<ParserStatement>,ParseError> {
    let pattern = parse_full_identifier(lexer,None)?;
    get_other(lexer,"(")?;
    let exprs = parse_exprlist(lexer,defstore,')',false)?;
    let pos = lexer.position();
    let identifier = defstore.pattern_to_identifier(lexer,&pattern,true).map_err(|e| ParseError::new(&e.to_string(),lexer))?;
    match defstore.get_stmt_id(&identifier.0) {
        Ok(s) => apply_macro(s,&exprs,lexer),
        Err(_) => {
            Ok(vec![ParserStatement::Regular(Statement(identifier.0,exprs,pos))])
        }
    }    
} 

fn parse_inlinestmt(lexer: &mut Lexer, defstore: &DefStore)-> Result<Vec<ParserStatement>,ParseError> {
    let left = parse_expr(lexer,defstore,false)?;
    let op = get_operator(lexer,false)?;
    let right = parse_expr(lexer,defstore,false)?;
    let inline = defstore.get_inline_binary(&op,lexer)?;
    if !defstore.stmt_like(&inline.identifier().0,lexer)? {
        Err(ParseError::new("Got inline expr, expected inline stmt",lexer))?;
    }
    let pos = lexer.position();
    Ok(vec![ParserStatement::Regular(Statement(inline.identifier().0.clone(),vec![left,right],pos))])
}

pub(in super) fn parse_statement(lexer: &mut Lexer, defstore: &DefStore, in_defn: bool) -> Result<Vec<ParserStatement>,ParseError> {
    let token = &lexer.peek(None,1)[0];
    match token {
        Token::Identifier(id) => {
            let mut need_semicolon = true;
            let out = match &id[..] {
                "module" => parse_module(lexer),
                "import" => parse_import(lexer),
                "inline" => parse_inline(lexer),
                "expr" => parse_exprdecl(lexer,defstore),
                "stmt" => { need_semicolon = false; parse_stmtdecl(lexer,defstore) },
                "func" => parse_func(lexer,defstore),
                "proc" => parse_proc(lexer,defstore),
                "struct" => parse_struct(lexer,defstore),
                "enum" => parse_enum(lexer,defstore),
                "use" => parse_use(lexer,defstore),
                x => {
                    id_not_reserved(&x.to_string(),lexer)?;
                    parse_regular(lexer,defstore)
                }
            }?;
            if need_semicolon {
                get_other(lexer,";")?;
            }
            Ok(out)
        },
        Token::EndOfFile => { lexer.get(); Ok(vec![]) },
        Token::Other('}') if in_defn => { Ok(vec![ParserStatement::EndOfBlock]) },
        Token::EndOfLex => Ok(vec![ParserStatement::EndOfParse]),
        _ => {
            let out = parse_regular(lexer,defstore)?;
            get_other(lexer,";")?;
            Ok(out)
        }            
    }
}
