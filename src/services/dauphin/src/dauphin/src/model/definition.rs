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
use crate::model::{ Identifier };

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
    identifier: Identifier,
    statement: bool,
    precedence: f64,
    mode: InlineMode
}

impl Inline {
    pub fn new(symbol: &str, identifier: &Identifier, statement: bool, precedence: f64, mode: &InlineMode) -> Inline {
        Inline {
            symbol: symbol.to_string(),
            identifier: identifier.clone(),
            statement, precedence, mode: *mode
        }
    }

    pub fn identifier(&self) -> &Identifier { &self.identifier }
    pub fn symbol(&self) -> &str { &self.symbol }
    pub fn precedence(&self) -> f64 { self.precedence }
    pub fn mode(&self) -> &InlineMode { &self.mode }
}

#[derive(Debug)]
pub struct ExprMacro {
    identifier: Identifier
}

impl ExprMacro {
    pub fn new(identifier: &Identifier) -> ExprMacro {
        ExprMacro { identifier: identifier.clone() }
    }

    pub fn identifier(&self) -> &Identifier { &self.identifier }
}

#[derive(Debug)]
pub struct StmtMacro {
    identifier: Identifier
}

impl StmtMacro {
    pub fn new(identifier: &Identifier) -> StmtMacro {
        StmtMacro { identifier: identifier.clone() }
    }

    pub fn identifier(&self) -> &Identifier { &self.identifier }
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
