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

use super::node::{ ParserStatement, ParseError };
use super::lexutil::not_reserved;
use crate::model::{
    InlineMode, Inline, DefStore, ExprMacro, StmtMacro, FuncDecl, ProcDecl,
    StructDef, EnumDef, IdentifierPattern, IdentifierGuesser
};
use crate::lexer::Lexer;
use crate::typeinf::{ SignatureConstraint, MemberType };

fn run_import(path: &str, lexer: &mut Lexer) -> Result<(),ParseError> {
    lexer.import(path).map_err(|s| ParseError::new(&format!("import failed: {}",s),lexer))
}

fn run_module(name: &str, lexer: &mut Lexer) -> Result<(),ParseError> {
    lexer.set_module(name);
    Ok(())
}

fn run_inline(symbol: &str, pattern: &IdentifierPattern, mode: &InlineMode, prio: f64, lexer: &mut Lexer, defstore: &mut DefStore, guesser: &mut IdentifierGuesser) -> Result<(),ParseError> {
    let identifier = guesser.add(lexer,pattern);
    let stmt_like = defstore.stmt_like(&identifier,lexer)?;
    lexer.add_inline(symbol,mode == &InlineMode::Prefix).map_err(|s| {
        ParseError::new(&s,lexer)
    })?;
    defstore.add_inline(Inline::new(symbol,&identifier,stmt_like,prio,mode))?;
    Ok(())
}

fn run_expr(pattern: &IdentifierPattern, defstore: &mut DefStore, lexer: &mut Lexer, guesser: &mut IdentifierGuesser) -> Result<(),ParseError> {
    let identifier = guesser.add(lexer,pattern);
    not_reserved(&identifier,lexer)?;
    defstore.add_expr(ExprMacro::new(&identifier),lexer)?;
    Ok(())
}

fn run_stmt(pattern: &IdentifierPattern, defstore: &mut DefStore, lexer: &mut Lexer, guesser: &mut IdentifierGuesser) -> Result<(),ParseError> {
    let identifier = guesser.add(lexer,pattern);
    not_reserved(&identifier,lexer)?;
    defstore.add_stmt(StmtMacro::new(&identifier),lexer)?;
    Ok(())
}

fn run_proc(pattern: &IdentifierPattern, signature: &SignatureConstraint, defstore: &mut DefStore, lexer: &mut Lexer, guesser: &mut IdentifierGuesser) -> Result<(),ParseError> {
    let identifier = guesser.add(lexer,pattern);
    not_reserved(&identifier,lexer)?;
    defstore.add_proc(ProcDecl::new(&identifier,signature),lexer)?;
    Ok(())
}

fn run_func(pattern: &IdentifierPattern, signature: &SignatureConstraint, defstore: &mut DefStore, lexer: &mut Lexer, guesser: &mut IdentifierGuesser) -> Result<(),ParseError> {
    let identifier = guesser.add(lexer,pattern);
    not_reserved(&identifier,lexer)?;
    defstore.add_func(FuncDecl::new(&identifier,signature),lexer)?;
    Ok(())
}

fn run_struct(pattern: &IdentifierPattern, member_types: &Vec<MemberType>, names: &Vec<String>, defstore: &mut DefStore, lexer: &mut Lexer, guesser: &mut IdentifierGuesser) -> Result<(),ParseError> {
    let identifier = guesser.add(lexer,pattern);
    not_reserved(&identifier,lexer)?;
    let def = StructDef::new(&identifier,member_types,names).map_err(|e| ParseError::new(&e,lexer) )?;
    defstore.add_struct(def,lexer)?;
    Ok(())
}

// TODO allow one operator as prefix of another
fn run_enum(pattern: &IdentifierPattern, member_types: &Vec<MemberType>, names: &Vec<String>, defstore: &mut DefStore, lexer: &mut Lexer, guesser: &mut IdentifierGuesser) -> Result<(),ParseError> {
    let identifier = guesser.add(lexer,pattern);
    not_reserved(&identifier,lexer)?;
    let def = EnumDef::new(&identifier,member_types,names).map_err(|e| ParseError::new(&e,lexer) )?;
    defstore.add_enum(def,lexer)?;
    Ok(())
}

pub fn declare(stmt: &ParserStatement, lexer: &mut Lexer, defstore: &mut DefStore, guesser: &mut IdentifierGuesser) -> Result<bool,ParseError> {
    match stmt {
        ParserStatement::Import(path) =>
            run_import(path,lexer).map(|_| true),
        ParserStatement::Module(name) =>
            run_module(name,lexer).map(|_| true),
        ParserStatement::Inline(symbol,pattern,mode,prio) => 
            run_inline(&symbol,&pattern,mode,*prio,lexer,defstore,guesser).map(|_| true),
        ParserStatement::ExprMacro(pattern) =>
            run_expr(&pattern,defstore,lexer,guesser).map(|_| true),
        ParserStatement::StmtMacro(pattern) =>
            run_stmt(&pattern,defstore,lexer,guesser).map(|_| true),
        ParserStatement::ProcDecl(pattern,signature) =>
            run_proc(pattern,&signature,defstore,lexer,guesser).map(|_| true),
        ParserStatement::FuncDecl(pattern,signature) =>
            run_func(pattern,signature,defstore,lexer,guesser).map(|_| true),
        ParserStatement::StructDef(pattern,member_types,names) =>
            run_struct(&pattern,&member_types,&names,defstore,lexer,guesser).map(|_| true),
        ParserStatement::EnumDef(pattern,member_types,names) =>
            run_enum(&pattern,&member_types,names,defstore,lexer,guesser).map(|_| true),
        _ => { return Ok(false); }
    }
}
