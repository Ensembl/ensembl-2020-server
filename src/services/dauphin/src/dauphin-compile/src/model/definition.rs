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

use crate::typeinf::SignatureConstraint;
use crate::model::{ IdentifierUse };
use crate::parser::{ Expression, Statement };
use dauphin_interp::command::Identifier;

#[derive(Debug,PartialEq,Clone,Copy)]
pub enum InlineMode {
    LeftAssoc,
    RightAssoc,
    Prefix,
    Suffix
}

#[derive(Debug)]
pub struct Inline {
    symbol: String,
    identifier: IdentifierUse,
    statement: bool,
    precedence: f64,
    mode: InlineMode
}

impl Inline {
    pub fn new(symbol: &str, identifier: &IdentifierUse, statement: bool, precedence: f64, mode: &InlineMode) -> Inline {
        Inline {
            symbol: symbol.to_string(),
            identifier: identifier.clone(),
            statement, precedence, mode: *mode
        }
    }

    pub fn identifier(&self) -> &IdentifierUse { &self.identifier }
    pub fn symbol(&self) -> &str { &self.symbol }
    pub fn precedence(&self) -> f64 { self.precedence }
    pub fn mode(&self) -> &InlineMode { &self.mode }
}

#[derive(Debug)]
pub struct ExprMacro {
    identifier: Identifier,
    args: Vec<Identifier>,
    expr: Expression
}

impl ExprMacro {
    pub fn new(identifier: &Identifier, args: Vec<Identifier>, expr: Expression) -> ExprMacro {
        ExprMacro { identifier: identifier.clone(), args, expr }
    }

    pub fn identifier(&self) -> &Identifier { &self.identifier }

    pub fn expression(&self, exprs: &[Expression]) -> Result<Expression,String> {
        self.expr.alpha(&self.args,exprs)
    }
}

#[derive(Debug)]
pub struct StmtMacro {
    identifier: Identifier,
    args: Vec<Identifier>,
    block: Vec<Statement>
}

impl StmtMacro {
    pub fn new(identifier: &Identifier, args: Vec<Identifier>, block: Vec<Statement>) -> StmtMacro {
        StmtMacro { identifier: identifier.clone(), args, block }
    }

    pub fn identifier(&self) -> &Identifier { &self.identifier }

    pub fn block(&self, exprs: &[Expression]) -> Result<Vec<Statement>,String> {
        self.block.iter().map(|x| x.alpha(&self.args,exprs)).collect()
    }
}

#[derive(Debug)]
pub struct FuncDecl {
    identifier: Identifier,
    signature: SignatureConstraint
}

impl FuncDecl {
    pub fn new(identifier: &Identifier, signature: &SignatureConstraint) -> FuncDecl {
        FuncDecl {
            identifier: identifier.clone(),
            signature: signature.clone()
        }
    }

    pub fn identifier(&self) -> &Identifier { &self.identifier }
    pub fn get_signature(&self) -> &SignatureConstraint { &self.signature }
}

#[derive(Debug)]
pub struct ProcDecl {
    identifier: Identifier,
    signature: SignatureConstraint
}

impl ProcDecl {
    pub fn new(identifier: &Identifier, signature: &SignatureConstraint) -> ProcDecl {
        ProcDecl { identifier: identifier.clone(), signature: signature.clone() }
    }

    pub fn identifier(&self) -> &Identifier { &self.identifier }
    pub fn get_signature(&self) -> &SignatureConstraint { &self.signature }
}
