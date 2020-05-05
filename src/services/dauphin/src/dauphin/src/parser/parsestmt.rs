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
use super::node::{ Statement, ParserStatement, ParseError };
use super::lexutil::{ get_string, get_other, id_not_reserved, get_identifier, get_number, get_operator };
use crate::model::{ DefStore, InlineMode, IdentifierGuesser };
use super::parsedecl::{ 
    parse_proc, parse_exprdecl, parse_stmtdecl, parse_func, parse_struct,
    parse_enum
};
use super::parseexpr::{ parse_expr, parse_exprlist, parse_full_identifier, peek_full_identifier };

fn parse_regular(lexer: &mut Lexer, defstore: &DefStore, guesser: &mut IdentifierGuesser) -> Result<ParserStatement,ParseError> {
    if let Some(pattern) = peek_full_identifier(lexer,None) {
        let identifier = guesser.guess(lexer,&pattern).map_err(|x| ParseError::new(&x.to_string(),lexer))?;
        if defstore.stmt_like(&identifier,lexer).unwrap_or(false) {
            return parse_funcstmt(lexer,defstore,guesser);
        }
    }
    parse_inlinestmt(lexer,defstore,guesser)
}

fn parse_import(lexer: &mut Lexer) -> Result<ParserStatement,ParseError> {
    lexer.get();
    Ok(ParserStatement::Import(get_string(lexer)?))
}

fn parse_module(lexer: &mut Lexer) -> Result<ParserStatement,ParseError> {
    lexer.get();
    Ok(ParserStatement::Module(get_string(lexer)?))
}

fn parse_inline(lexer: &mut Lexer) -> Result<ParserStatement,ParseError> {
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
    let prio = get_number(lexer)?;
    Ok(ParserStatement::Inline(symbol,pattern,mode,prio))
}

fn parse_funcstmt(lexer: &mut Lexer, defstore: &DefStore, guesser: &mut IdentifierGuesser)-> Result<ParserStatement,ParseError> {
    let pattern = parse_full_identifier(lexer,None)?;
    get_other(lexer,"(")?;
    let exprs = parse_exprlist(lexer,defstore,guesser,')',false)?;
    let (file,line,_) = lexer.position();
    let identifier = guesser.guess(lexer,&pattern).map_err(|e| ParseError::new(&e.to_string(),lexer))?;
    Ok(ParserStatement::Regular(Statement(identifier,exprs,file.to_string(),line)))
} 

fn parse_inlinestmt(lexer: &mut Lexer, defstore: &DefStore, guesser: &mut IdentifierGuesser)-> Result<ParserStatement,ParseError> {
    let left = parse_expr(lexer,defstore,guesser,false)?;
    let op = get_operator(lexer,false)?;
    let right = parse_expr(lexer,defstore,guesser,false)?;
    let inline = defstore.get_inline_binary(&op,lexer)?;
    if !defstore.stmt_like(&inline.identifier(),lexer)? {
        Err(ParseError::new("Got inline expr, expected inline stmt",lexer))?;
    }
    let (file,line,_) = lexer.position();
    Ok(ParserStatement::Regular(Statement(inline.identifier().clone(),vec![left,right],file.to_string(),line)))
}

pub(in super) fn parse_statement(lexer: &mut Lexer, defstore: &DefStore, guesser: &mut IdentifierGuesser) -> Result<Option<ParserStatement>,ParseError> {
    let token = &lexer.peek(None,1)[0];
    match token {
        Token::Identifier(id) => {
            let out = match &id[..] {
                "module" => parse_module(lexer),
                "import" => parse_import(lexer),
                "inline" => parse_inline(lexer),
                "expr" => parse_exprdecl(lexer),
                "stmt" => parse_stmtdecl(lexer),
                "func" => parse_func(lexer,defstore,guesser),
                "proc" => parse_proc(lexer,defstore,guesser),
                "struct" => parse_struct(lexer,defstore,guesser),
                "enum" => parse_enum(lexer,defstore,guesser),
                x => {
                    id_not_reserved(&x.to_string(),lexer)?;
                    parse_regular(lexer,defstore,guesser)
                }
            }?;
            get_other(lexer,";")?;
            Ok(Some(out))
        },
        Token::EndOfFile => { lexer.get(); Ok(None) },
        Token::EndOfLex => Ok(Some(ParserStatement::EndOfParse)),
        _ => {
            let out = parse_regular(lexer,defstore,guesser)?;
            get_other(lexer,";")?;
            Ok(Some(out))
        }            
    }
}
