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
 *  
 *  vscode-fold=1
 */

use crate::lexer::{ Lexer, Token };
use super::node::{ Statement, ParserStatement, ParseError };
use super::lexutil::{ get_string, get_other, not_reserved, get_identifier, get_number, get_operator };
use crate::model::{ DefStore, InlineMode };
use super::parsedecl::{ 
    parse_proc, parse_exprdecl, parse_stmtdecl, parse_func, parse_struct,
    parse_enum
};
use super::parseexpr::{ parse_expr, parse_exprlist };

fn parse_regular(lexer: &mut Lexer, defstore: &DefStore) -> Result<ParserStatement,ParseError> {
    if let Token::Identifier(id) = lexer.peek() {
        let id = id.clone();
        if defstore.stmt_like(&id,lexer).unwrap_or(false) {
            return parse_funcstmt(lexer,defstore);
        }
    }
    parse_inlinestmt(lexer,defstore)
}

fn parse_import(lexer: &mut Lexer) -> Result<ParserStatement,ParseError> {
    lexer.get();
    Ok(ParserStatement::Import(get_string(lexer)?))
}

fn parse_inline(lexer: &mut Lexer) -> Result<ParserStatement,ParseError> {
    lexer.get();
    let symbol = get_string(lexer)?;
    let name = get_identifier(lexer)?;
    let mode = match &get_identifier(lexer)?[..] {
        "left" => Ok(InlineMode::LeftAssoc),
        "right" => Ok(InlineMode::RightAssoc),
        "prefix" => Ok(InlineMode::Prefix),
        "suffix" => Ok(InlineMode::Suffix),
        _ => Err(ParseError::new("Bad oper mode, expected left, right, prefix, or suffix",lexer))
    }?;
    let prio = get_number(lexer)?;
    Ok(ParserStatement::Inline(symbol,name,mode,prio))
}

fn parse_funcstmt(lexer: &mut Lexer, defstore: &DefStore)-> Result<ParserStatement,ParseError> {
    let name = get_identifier(lexer)?;
    get_other(lexer,"(")?;
    let exprs = parse_exprlist(lexer,defstore,')',false)?;
    let (file,line,_) = lexer.position();
    Ok(ParserStatement::Regular(Statement(name,exprs,file.to_string(),line)))
} 

fn parse_inlinestmt(lexer: &mut Lexer, defstore: &DefStore)-> Result<ParserStatement,ParseError> {
    let left = parse_expr(lexer,defstore, false)?;
    let op = get_operator(lexer,false)?;
    let right = parse_expr(lexer,defstore,false)?;
    let name = defstore.get_inline_binary(&op,lexer)?.name();
    if !defstore.stmt_like(&name,lexer)? {
        Err(ParseError::new("Got inline expr, expected inline stmt",lexer))?;
    }
    let (file,line,_) = lexer.position();
    Ok(ParserStatement::Regular(Statement(name.to_string(),vec![left,right],file.to_string(),line)))
}

pub(in super) fn parse_statement(lexer: &mut Lexer, defstore: &DefStore) -> Result<Option<ParserStatement>,ParseError> {
    let token = lexer.peek();
    match token {
        Token::Identifier(id) => {
            let out = match &id[..] {
                "import" => parse_import(lexer),
                "inline" => parse_inline(lexer),
                "expr" => parse_exprdecl(lexer),
                "stmt" => parse_stmtdecl(lexer),
                "func" => parse_func(lexer,defstore),
                "proc" => parse_proc(lexer,defstore),
                "struct" => parse_struct(lexer,defstore),
                "enum" => parse_enum(lexer,defstore),
                x => {
                    not_reserved(&x.to_string(),lexer)?;
                    parse_regular(lexer,defstore)
                }
            }?;
            get_other(lexer,";")?;
            Ok(Some(out))
        },
        Token::EndOfFile => { lexer.get(); Ok(None) },
        Token::EndOfLex => Ok(Some(ParserStatement::EndOfParse)),
        _ => {
            let out = parse_regular(lexer,defstore)?;
            get_other(lexer,";")?;
            Ok(Some(out))
        }            
    }
}
