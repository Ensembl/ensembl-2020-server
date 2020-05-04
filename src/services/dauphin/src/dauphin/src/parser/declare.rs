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
    StructDef, EnumDef
};
use crate::lexer::Lexer;
use crate::typeinf::{ SignatureConstraint, MemberType };

fn run_import(path: &str, lexer: &mut Lexer) -> Result<(),ParseError> {
    lexer.import(path).map_err(|s| ParseError::new(&format!("import failed: {}",s),lexer))
}

fn run_inline(symbol: &str, module: &Option<String>, name: &str, mode: &InlineMode, prio: f64, lexer: &mut Lexer, defstore: &mut DefStore) -> Result<(),ParseError> {
    let module = module.as_ref().map(|x| x as &str).unwrap_or(""); // XXX module
    let stmt_like = defstore.stmt_like(Some(module),name,lexer)?;
    lexer.add_inline(symbol,mode == &InlineMode::Prefix).map_err(|s| {
        ParseError::new(&s,lexer)
    })?;
    defstore.add_inline(Inline::new(symbol,module,name,stmt_like,prio,mode))?;
    Ok(())
}

fn run_expr(name: &str, defstore: &mut DefStore, lexer: &mut Lexer) -> Result<(),ParseError> {
    not_reserved(name,lexer)?;
    defstore.add_expr(ExprMacro::new(name),lexer)?;
    Ok(())
}

fn run_stmt(name: &str, defstore: &mut DefStore, lexer: &mut Lexer) -> Result<(),ParseError> {
    not_reserved(name,lexer)?;
    defstore.add_stmt(StmtMacro::new(name),lexer)?;
    Ok(())
}

fn run_proc(module: &Option<String>, name: &str, signature: &SignatureConstraint, defstore: &mut DefStore, lexer: &mut Lexer) -> Result<(),ParseError> {
    not_reserved(name,lexer)?;
    let module = module.as_ref().map(|x| x as &str).unwrap_or(""); // XXX module
    defstore.add_proc(ProcDecl::new(module,name,signature),lexer)?;
    Ok(())
}

fn run_func(module: &Option<String>, name: &str, signature: &SignatureConstraint, defstore: &mut DefStore, lexer: &mut Lexer) -> Result<(),ParseError> {
    not_reserved(name,lexer)?;
    let module = module.as_ref().map(|x| x as &str).unwrap_or(""); // XXX module
    defstore.add_func(FuncDecl::new(module,name,signature),lexer)?;
    Ok(())
}

fn run_struct(name: &str, member_types: &Vec<MemberType>, names: &Vec<String>, defstore: &mut DefStore, lexer: &mut Lexer) -> Result<(),ParseError> {
    not_reserved(name,lexer)?;
    let def = StructDef::new(name,member_types,names).map_err(|e| ParseError::new(&e,lexer) )?;
    defstore.add_struct(def,lexer)?;
    Ok(())
}

// TODO allow one operator as prefix of another
fn run_enum(name: &str, member_types: &Vec<MemberType>, names: &Vec<String>, defstore: &mut DefStore, lexer: &mut Lexer) -> Result<(),ParseError> {
    not_reserved(name,lexer)?;
    let def = EnumDef::new(name,member_types,names).map_err(|e| ParseError::new(&e,lexer) )?;
    defstore.add_enum(def,lexer)?;
    Ok(())
}

pub fn declare(stmt: &ParserStatement, lexer: &mut Lexer, defstore: &mut DefStore) -> Result<bool,ParseError> {
    match stmt {
        ParserStatement::Import(path) =>
            run_import(path,lexer).map(|_| true),
        ParserStatement::Inline(symbol,module,name,mode,prio) => 
            run_inline(&symbol,&module,&name,mode,*prio,lexer,defstore).map(|_| true),
        ParserStatement::ExprMacro(name) =>
            run_expr(&name,defstore,lexer).map(|_| true),
        ParserStatement::StmtMacro(name) =>
            run_stmt(&name,defstore,lexer).map(|_| true),
        ParserStatement::ProcDecl(module,name,signature) =>
            run_proc(&module,&name,&signature,defstore,lexer).map(|_| true),
        ParserStatement::FuncDecl(module,name,signature) =>
            run_func(&module,&name,signature,defstore,lexer).map(|_| true),
        ParserStatement::StructDef(name,member_types,names) =>
            run_struct(&name,&member_types,&names,defstore,lexer).map(|_| true),
        ParserStatement::EnumDef(name,member_types,names) =>
            run_enum(&name,&member_types,names,defstore,lexer).map(|_| true),
        _ => { return Ok(false); }
    }
}
