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

use std::collections::HashMap;
use std::collections::hash_map::Entry;
use super::definition::{
    ExprMacro, StmtMacro, FuncDecl, ProcDecl, Inline, InlineMode
};
use super::structenum::{ StructDef, EnumDef };
use super::identifierstore::{ IdentifierStore, IdentifierStoreError };
use crate::lexer::Lexer;
use crate::parser::ParseError;

#[derive(Debug)]
pub struct DefStore {
    namespace: HashMap<String,(String,u32,u32)>,
    exprs: HashMap<String,ExprMacro>,
    stmts: HashMap<String,StmtMacro>,
    funcs: IdentifierStore<FuncDecl>,
    procs: IdentifierStore<ProcDecl>,
    structs: HashMap<String,StructDef>,
    enums: HashMap<String,EnumDef>,
    inlines_binary: HashMap<String,Inline>,
    inlines_unary: HashMap<String,Inline>,
    structenum_order: Vec<String>
}

impl DefStore {
    pub fn new() -> DefStore {
        DefStore {
            namespace: HashMap::new(),
            exprs: HashMap::new(),
            stmts: HashMap::new(),
            funcs: IdentifierStore::new(),
            procs: IdentifierStore::new(),
            structs: HashMap::new(),
            enums: HashMap::new(),
            inlines_binary: HashMap::new(),
            inlines_unary: HashMap::new(),
            structenum_order: Vec::new()
        }
    }

    fn detect_clash(&mut self, cmp: &str, lexer: &Lexer) -> Result<(),ParseError> {
        match self.namespace.entry(cmp.to_string()) {
            Entry::Occupied(e) => {
                let (file,line,col) = e.get();
                Err(ParseError::new(
                    &format!("'{}' already defined at {} {}:{}",
                        cmp,file,line,col),lexer))
            },
            Entry::Vacant(e) => {
                let (file,line,col) = lexer.position();
                e.insert((file.to_string(),line,col));
                Ok(())
            }
        }
    }

    pub fn add_expr(&mut self, expr: ExprMacro, lexer: &Lexer) -> Result<(),ParseError> {
        self.detect_clash(expr.name(),lexer)?;
        self.exprs.insert(expr.name().to_string(),expr);
        Ok(())
    }

    pub fn add_stmt(&mut self, stmt: StmtMacro, lexer: &Lexer) -> Result<(),ParseError> {
        self.detect_clash(stmt.name(),lexer)?;
        self.stmts.insert(stmt.name().to_string(),stmt);
        Ok(())
    }

    pub fn add_func(&mut self, func: FuncDecl, lexer: &Lexer) -> Result<(),ParseError> {
        self.detect_clash(func.name(),lexer)?;
        self.funcs.add(&func.module().to_string(),&func.name().to_string(),func);
        Ok(())
    }

    pub fn add_proc(&mut self, proc_: ProcDecl, lexer: &Lexer) -> Result<(),ParseError> {
        self.detect_clash(proc_.name(),lexer)?;
        self.procs.add(&proc_.module().to_string(),&proc_.name().to_string(),proc_);
        Ok(())
    }

    pub fn add_struct(&mut self, struct_: StructDef, lexer: &Lexer) -> Result<(),ParseError> {
        self.detect_clash(struct_.name(),lexer)?;
        self.structenum_order.push(struct_.name().to_string());
        self.structs.insert(struct_.name().to_string(),struct_);
        Ok(())
    }

    pub fn get_struct(&self, name: &str) -> Option<&StructDef> {
        self.structs.get(name)
    }

    pub fn add_enum(&mut self, enum_: EnumDef, lexer: &Lexer) -> Result<(),ParseError> {
        self.detect_clash(enum_.name(),lexer)?;
        self.structenum_order.push(enum_.name().to_string());
        self.enums.insert(enum_.name().to_string(),enum_);
        Ok(())
    }

    pub fn get_enum(&self, name: &str) -> Option<&EnumDef> {
        self.enums.get(name)
    }

    pub fn add_inline(&mut self, inline: Inline) -> Result<(),ParseError> {
        if inline.mode() == &InlineMode::Prefix {
            self.inlines_unary.insert(inline.symbol().to_string(),inline);
        } else {
            self.inlines_binary.insert(inline.symbol().to_string(),inline);
        }
        Ok(())
    }

    pub fn get_inline_binary(&self, symbol: &str, lexer: &Lexer) -> Result<&Inline,ParseError> {
        self.inlines_binary.get(symbol).ok_or(
            ParseError::new(&format!("No such binary operator: {}",symbol),lexer)
        )
    }

    pub fn get_inline_unary(&self, symbol: &str, lexer: &Lexer) -> Result<&Inline,ParseError> {
        self.inlines_unary.get(symbol).ok_or(
            ParseError::new(&format!("No such unary operator: {}",symbol),lexer)
        )
    }

    pub fn stmt_like(&self, module: Option<&str>, name: &str, lexer: &Lexer) -> Result<bool,ParseError> {
        if self.stmts.contains_key(name) || self.procs.contains_key(module,name) {
            Ok(true)
        } else if self.exprs.contains_key(name) || self.funcs.contains_key(module,name) {
            Ok(false)
        } else {
            Err(ParseError::new(&format!("Missing or ambiguous symbol: '{}'",name),lexer))
        }
    }

    pub fn get_func(&self, module: Option<&str>, name: &str) -> Result<&FuncDecl,IdentifierStoreError> {
        self.funcs.get(module,name).map(|x| x.1)
    }

    pub fn get_proc(&self, module: Option<&str>, name: &str) -> Result<&ProcDecl,IdentifierStoreError> {
        self.procs.get(module,name).map(|x| x.1)
    }

    pub fn get_structenum_order(&self) -> impl DoubleEndedIterator<Item=&String> {
        print!("structenumorder = {:?}\n",self.structenum_order);
        self.structenum_order.iter()
    }
}
