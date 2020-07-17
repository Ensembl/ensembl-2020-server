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
use super::definition::{
    ExprMacro, StmtMacro, FuncDecl, ProcDecl, Inline, InlineMode
};
use super::structenum::{ StructDef, EnumDef };
use super::identifierstore::{ IdentifierStore, IdentifierPattern, IdentifierUse };
use crate::lexer::Lexer;
use crate::parser::ParseError;
use dauphin_interp::command::Identifier;

#[derive(Debug)]
pub enum IdentifierValue {
    Expr(ExprMacro),
    Stmt(StmtMacro),
    Func(FuncDecl),
    Proc(ProcDecl),
    Struct(StructDef),
    Enum(EnumDef)
}

#[derive(Debug)]
pub struct DefStore {
    source: String,
    identifiers: IdentifierStore<IdentifierValue>,
    inlines_binary: HashMap<String,Inline>,
    inlines_unary: HashMap<String,Inline>,
    structenum_order: Vec<Identifier>
}

macro_rules! accessor {
    ($accessor:ident,$setter:ident,$branch:tt,$type:ty,$name:expr) => {
        pub fn $accessor(&self, identifier: &Identifier) -> Result<&$type,String> {
            if let IdentifierValue::$branch(out) = self.get_id(identifier)? {
                Ok(out)
            } else {
                Err(format!("{} is not a {}",identifier,$name))
            }
        }

        pub fn $setter(&mut self, data: $type, lexer: &Lexer) -> Result<(),ParseError> {
            let id = data.identifier().clone();
            self.detect_clash(&id,lexer)?;
            let data = IdentifierValue::$branch(data);
            match data {
                IdentifierValue::Struct(_) | IdentifierValue::Enum(_) => {
                    self.structenum_order.push(id.clone());
                },
                _ => {}
            }
            self.identifiers.add(&id,data);
            Ok(())
        }
    };
}
impl DefStore {
    pub fn new(source: &str) -> DefStore {
        DefStore {
            source: source.to_string(),
            identifiers: IdentifierStore::new(),
            inlines_binary: HashMap::new(),
            inlines_unary: HashMap::new(),
            structenum_order: Vec::new()
        }
    }

    pub fn get_source(&self) -> &str { &self.source }

    fn detect_clash(&mut self, identifier: &Identifier, lexer: &Lexer) -> Result<(),ParseError> {
        if self.identifiers.contains_key(identifier) {
            Err(ParseError::new(&format!("duplicate identifier: {}",identifier),lexer))
        } else {
            Ok(())
        }
    }

    fn get_id(&self, identifier: &Identifier) -> Result<&IdentifierValue,String> {
        self.identifiers.get_id(identifier)
    }

    pub fn pattern_to_identifier(&self, lexer: &Lexer, pattern: &IdentifierPattern, guess: bool) -> Result<IdentifierUse,String> {
        if let Some(first) = &pattern.0 {
            return Ok(IdentifierUse(Identifier::new(first,&pattern.1),false));
        } else if guess {
            let mut module = None;
            for short in lexer.get_shorts().iter() {
                if self.identifiers.contains_key(&Identifier::new(short,&pattern.1)) {
                    if module.is_some() {
                        return Err(format!("duplicate match for identifier '{}': use :: syntax",pattern.1))
                    } else {
                        module = Some(short);
                    }
                }
            }
            if let Some(module) = module {
                return Ok(IdentifierUse(Identifier::new(module,&pattern.1),true));
            }
        }
        Ok(IdentifierUse(Identifier::new(lexer.get_module(),&pattern.1),true))
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

    pub fn stmt_like(&self, identifier: &Identifier, lexer: &Lexer) -> Result<bool,ParseError> {
        match self.get_id(identifier) {
            Ok(IdentifierValue::Stmt(_)) | Ok(IdentifierValue::Proc(_)) => Ok(true),
            Ok(IdentifierValue::Expr(_)) | Ok(IdentifierValue::Func(_)) => Ok(false),
            _ => Err(ParseError::new(&format!("Missing or ambiguous symbol: '{}'",identifier),lexer))
        }
    }

    pub fn get_structenum_order(&self) -> impl DoubleEndedIterator<Item=&Identifier> {
        self.structenum_order.iter()
    }

    accessor!(get_struct_id,add_struct,Struct,StructDef,"struct");
    accessor!(get_enum_id,add_enum,Enum,EnumDef,"enum");
    accessor!(get_proc_id,add_proc,Proc,ProcDecl,"proc");
    accessor!(get_func_id,add_func,Func,FuncDecl,"func");
    accessor!(get_expr_id,add_expr,Expr,ExprMacro,"expr");
    accessor!(get_stmt_id,add_stmt,Stmt,StmtMacro,"stmt");
}
