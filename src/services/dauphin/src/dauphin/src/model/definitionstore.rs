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
use super::identifierstore::{ IdentifierStore, IdentifierStoreError, IdentifierPattern, Identifier };
use crate::lexer::Lexer;
use crate::parser::ParseError;

#[derive(Debug)]
pub struct DefStore {
    namespace: HashMap<Identifier,(String,u32,u32)>,
    exprs: IdentifierStore<ExprMacro>,
    stmts: IdentifierStore<StmtMacro>,
    funcs: IdentifierStore<FuncDecl>,
    procs: IdentifierStore<ProcDecl>,
    structs: IdentifierStore<StructDef>,
    enums: IdentifierStore<EnumDef>,
    inlines_binary: HashMap<String,Inline>,
    inlines_unary: HashMap<String,Inline>,
    structenum_order: Vec<Identifier>
}

impl DefStore {
    pub fn new() -> DefStore {
        DefStore {
            namespace: HashMap::new(),
            exprs: IdentifierStore::new(),
            stmts: IdentifierStore::new(),
            funcs: IdentifierStore::new(),
            procs: IdentifierStore::new(),
            structs: IdentifierStore::new(),
            enums: IdentifierStore::new(),
            inlines_binary: HashMap::new(),
            inlines_unary: HashMap::new(),
            structenum_order: Vec::new()
        }
    }

    fn detect_clash(&mut self, identifier: &Identifier, lexer: &Lexer) -> Result<(),ParseError> {
        match self.namespace.entry(identifier.clone()) {
            Entry::Occupied(e) => {
                let (file,line,col) = e.get();
                Err(ParseError::new(
                    &format!("'{}' already defined at {} {}:{}",
                        identifier,file,line,col),lexer))
            },
            Entry::Vacant(e) => {
                let (file,line,col) = lexer.position();
                e.insert((file.to_string(),line,col));
                Ok(())
            }
        }
    }

    pub fn add_expr(&mut self, expr: ExprMacro, lexer: &Lexer) -> Result<(),ParseError> {
        self.detect_clash(expr.identifier(),lexer)?;
        self.exprs.add(&expr.identifier().clone(),expr);
        Ok(())
    }

    pub fn add_stmt(&mut self, stmt: StmtMacro, lexer: &Lexer) -> Result<(),ParseError> {
        self.detect_clash(stmt.identifier(),lexer)?;
        self.stmts.add(&stmt.identifier().clone(),stmt);
        Ok(())
    }

    pub fn add_func(&mut self, func: FuncDecl, lexer: &Lexer) -> Result<(),ParseError> {
        self.detect_clash(func.identifier(),lexer)?;
        self.funcs.add(&func.identifier().clone(),func);
        Ok(())
    }

    pub fn add_proc(&mut self, proc_: ProcDecl, lexer: &Lexer) -> Result<(),ParseError> {
        self.detect_clash(proc_.identifier(),lexer)?;
        self.procs.add(&proc_.identifier().clone(),proc_);
        Ok(())
    }

    pub fn add_struct(&mut self, struct_: StructDef, lexer: &Lexer) -> Result<(),ParseError> {
        self.detect_clash(struct_.identifier(),lexer)?;
        self.structenum_order.push(struct_.identifier().clone());
        self.structs.add(&struct_.identifier().clone(),struct_);
        Ok(())
    }

    pub fn get_struct_id(&self, identifier: &Identifier) -> Result<&StructDef,IdentifierStoreError> {
        self.structs.get_id(identifier)
    }

    pub fn add_enum(&mut self, enum_: EnumDef, lexer: &Lexer) -> Result<(),ParseError> {
        self.detect_clash(enum_.identifier(),lexer)?;
        self.structenum_order.push(enum_.identifier().clone());
        self.enums.add(&enum_.identifier().clone(),enum_);
        Ok(())
    }

    pub fn get_enum_id(&self, identifier: &Identifier) -> Result<&EnumDef,IdentifierStoreError> {
        self.enums.get_id(identifier)
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

    pub fn stmt_like(&self, pattern: &IdentifierPattern, lexer: &Lexer) -> Result<bool,ParseError> {
        if self.stmts.contains_key(pattern) || self.procs.contains_key(pattern) {
            Ok(true)
        } else if self.exprs.contains_key(pattern) || self.funcs.contains_key(pattern) {
            Ok(false)
        } else {
            Err(ParseError::new(&format!("Missing or ambiguous symbol: '{}'",pattern),lexer))
        }
    }

    pub fn get_proc_id(&self, identifier: &Identifier) -> Result<&ProcDecl,IdentifierStoreError> {
        self.procs.get_id(identifier)
    }

    pub fn get_func_id(&self, identifier: &Identifier) -> Result<&FuncDecl,IdentifierStoreError> {
        self.funcs.get_id(identifier)
    }

    pub fn get_structenum_order(&self) -> impl DoubleEndedIterator<Item=&Identifier> {
        self.structenum_order.iter()
    }
}
