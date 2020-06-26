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

use super::node::{ ParserStatement, ParseError, Statement, Expression };
use super::lexutil::not_reserved;
use crate::model::{
    InlineMode, Inline, DefStore, ExprMacro, StmtMacro, FuncDecl, ProcDecl,
    StructDef, EnumDef, IdentifierPattern
};
use crate::lexer::Lexer;
use crate::typeinf::{ SignatureConstraint, MemberType };

fn run_import(path: &str, lexer: &mut Lexer) -> Result<(),ParseError> {
    lexer.import(path).map_err(|s| ParseError::new(&format!("import of {} failed: {}",path,s),lexer))
}

fn run_use(name: &str, lexer: &mut Lexer) -> Result<(),ParseError> {
    lexer.add_short(name);
    Ok(())
}

fn run_module(name: &str, lexer: &mut Lexer) -> Result<(),ParseError> {
    lexer.set_module(name);
    Ok(())
}

fn run_inline(symbol: &str, pattern: &IdentifierPattern, mode: &InlineMode, prio: f64, lexer: &mut Lexer, defstore: &mut DefStore) -> Result<(),ParseError> {
    let identifier = defstore.pattern_to_identifier(lexer,&pattern,false).map_err(|e| ParseError::new(&e.to_string(),lexer))?;
    let stmt_like = defstore.stmt_like(&identifier.0,lexer)?;
    lexer.add_inline(symbol,mode == &InlineMode::Prefix).map_err(|s| {
        ParseError::new(&s,lexer)
    })?;
    defstore.add_inline(Inline::new(symbol,&identifier,stmt_like,prio,mode))?;
    Ok(())
}

fn run_expr(pattern: &IdentifierPattern, args_in: &[IdentifierPattern], expr: &Expression, defstore: &mut DefStore, lexer: &mut Lexer) -> Result<(),ParseError> {
    let identifier = defstore.pattern_to_identifier(lexer,&pattern,false).map_err(|e| ParseError::new(&e.to_string(),lexer))?;
    let mut args = vec![];
    for pattern in args_in.iter() {
        args.push(defstore.pattern_to_identifier(lexer,&pattern,false).map_err(|e| ParseError::new(&e.to_string(),lexer))?.0);
    }
    not_reserved(&identifier.0,lexer)?;
    defstore.add_expr(ExprMacro::new(&identifier.0,args,expr.clone()),lexer)?;
    Ok(())
}

fn run_stmt(pattern: &IdentifierPattern, args_in: &[IdentifierPattern], block: &[Statement], defstore: &mut DefStore, lexer: &mut Lexer) -> Result<(),ParseError> {
    let identifier = defstore.pattern_to_identifier(lexer,&pattern,false).map_err(|e| ParseError::new(&e.to_string(),lexer))?;
    let mut args = vec![];
    for pattern in args_in.iter() {
        args.push(defstore.pattern_to_identifier(lexer,&pattern,false).map_err(|e| ParseError::new(&e.to_string(),lexer))?.0);
    }
    not_reserved(&identifier.0,lexer)?;
    defstore.add_stmt(StmtMacro::new(&identifier.0,args,block.to_vec()),lexer)?;
    Ok(())
}

fn run_proc(pattern: &IdentifierPattern, signature: &SignatureConstraint, defstore: &mut DefStore, lexer: &mut Lexer) -> Result<(),ParseError> {
    let identifier = defstore.pattern_to_identifier(lexer,&pattern,false).map_err(|e| ParseError::new(&e.to_string(),lexer))?;
    not_reserved(&identifier.0,lexer)?;
    defstore.add_proc(ProcDecl::new(&identifier.0,signature),lexer)?;
    Ok(())
}

fn run_func(pattern: &IdentifierPattern, signature: &SignatureConstraint, defstore: &mut DefStore, lexer: &mut Lexer) -> Result<(),ParseError> {
    let identifier = defstore.pattern_to_identifier(lexer,&pattern,false).map_err(|e| ParseError::new(&e.to_string(),lexer))?;
    not_reserved(&identifier.0,lexer)?;
    defstore.add_func(FuncDecl::new(&identifier.0,signature),lexer)?;
    Ok(())
}

fn run_struct(pattern: &IdentifierPattern, member_types: &Vec<MemberType>, names: &Vec<String>, defstore: &mut DefStore, lexer: &mut Lexer) -> Result<(),ParseError> {
    let identifier = defstore.pattern_to_identifier(lexer,&pattern,false).map_err(|e| ParseError::new(&e.to_string(),lexer))?;
    not_reserved(&identifier.0,lexer)?;
    let def = StructDef::new(&identifier.0,member_types,names).map_err(|e| ParseError::new(&e,lexer) )?;
    defstore.add_struct(def,lexer)?;
    Ok(())
}

// TODO allow one operator as prefix of another
fn run_enum(pattern: &IdentifierPattern, member_types: &Vec<MemberType>, names: &Vec<String>, defstore: &mut DefStore, lexer: &mut Lexer) -> Result<(),ParseError> {
    let identifier = defstore.pattern_to_identifier(lexer,&pattern,false).map_err(|e| ParseError::new(&e.to_string(),lexer))?;
    not_reserved(&identifier.0,lexer)?;
    let def = EnumDef::new(&identifier.0,member_types,names).map_err(|e| ParseError::new(&e,lexer) )?;
    defstore.add_enum(def,lexer)?;
    Ok(())
}

pub fn declare(stmt: &ParserStatement, lexer: &mut Lexer, defstore: &mut DefStore) -> Result<bool,ParseError> {
    match stmt {
        ParserStatement::Import(path) =>
            run_import(path,lexer).map(|_| true),
        ParserStatement::Use(name) =>
            run_use(name,lexer).map(|_| true),
        ParserStatement::Module(name) =>
            run_module(name,lexer).map(|_| true),
        ParserStatement::Inline(symbol,pattern,mode,prio) => 
            run_inline(&symbol,&pattern,mode,*prio,lexer,defstore).map(|_| true),
        ParserStatement::ExprMacro(pattern,args,expr) =>
            run_expr(&pattern,args,expr,defstore,lexer).map(|_| true),
        ParserStatement::StmtMacro(pattern,args,block) =>
            run_stmt(&pattern,args,block,defstore,lexer).map(|_| true),
        ParserStatement::ProcDecl(pattern,signature) =>
            run_proc(pattern,&signature,defstore,lexer).map(|_| true),
        ParserStatement::FuncDecl(pattern,signature) =>
            run_func(pattern,signature,defstore,lexer).map(|_| true),
        ParserStatement::StructDef(pattern,member_types,names) =>
            run_struct(&pattern,&member_types,&names,defstore,lexer).map(|_| true),
        ParserStatement::EnumDef(pattern,member_types,names) =>
            run_enum(&pattern,&member_types,names,defstore,lexer).map(|_| true),
        _ => { return Ok(false); }
    }
}
