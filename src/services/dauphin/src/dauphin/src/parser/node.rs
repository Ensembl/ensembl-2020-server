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

use std::fmt;
use hex;

use crate::model::{ InlineMode, IdentifierPattern, Identifier };
use crate::lexer::Lexer;
use crate::typeinf::{ MemberType, SignatureConstraint };

#[derive(PartialEq)]
pub enum Expression {
    Identifier(String),
    Number(f64),
    LiteralString(String),
    LiteralBytes(Vec<u8>),
    LiteralBool(bool),
    Operator(Identifier,Vec<Expression>),
    Star(Box<Expression>),
    Square(Box<Expression>),
    Bracket(Box<Expression>,Box<Expression>),
    Filter(Box<Expression>,Box<Expression>),
    Dot(Box<Expression>,String),
    Query(Box<Expression>,String),
    Pling(Box<Expression>,String),
    Vector(Vec<Expression>),
    CtorStruct(Identifier,Vec<Expression>,Vec<String>),
    CtorEnum(Identifier,String,Box<Expression>),
    Dollar,
    At
}

fn write_csl(f: &mut fmt::Formatter<'_>, exprs: &Vec<Expression>) -> fmt::Result {
    for (i,sub) in exprs.iter().enumerate() {
        if i > 0 {
            write!(f,",")?;
        }
        write!(f,"{:?}",sub)?;
    }
    Ok(())
}

fn write_csl_named(f: &mut fmt::Formatter<'_>, exprs: &Vec<Expression>, names: &Vec<String>) -> fmt::Result {
    let mut names = names.iter();
    for (i,sub) in exprs.iter().enumerate() {
        let name = names.next().unwrap();
        if i > 0 {
            write!(f,",")?;
        }
        write!(f,"{}: {:?}",name,sub)?;
    }
    Ok(())
}

impl fmt::Debug for Expression {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Expression::Identifier(id) => write!(f,"{}",id),
            Expression::Number(n) => write!(f,"{}",n),
            Expression::LiteralString(s) => write!(f,"{:?}",s),
            Expression::LiteralBytes(b) => write!(f,"'{}'",hex::encode(b)),
            Expression::LiteralBool(b) => write!(f,"{}",if *b { "true" } else {"false"}),
            Expression::Star(s) => write!(f,"*({:?})",s),
            Expression::Square(s) => write!(f,"({:?})[]",s),
            Expression::Bracket(left,inner) => write!(f,"({:?})[{:?}]",left,inner),
            Expression::Filter(left,inner) => write!(f,"({:?})&[{:?}]",left,inner),
            Expression::Dot(expr,key) => write!(f,"{:?}.{}",expr,key),
            Expression::Query(expr,key) => write!(f,"{:?}?{}",expr,key),
            Expression::Pling(expr,key) => write!(f,"{:?}!{}",expr,key),
            Expression::Dollar => write!(f,"$"),
            Expression::At => write!(f,"@"),
            Expression::Vector(x) => {
                write!(f,"[")?;
                write_csl(f,x)?;
                write!(f,"]")?;
                Ok(())
            }
            Expression::Operator(s,x) => {
                write!(f,"{}(",s.name())?; // XXX module
                write_csl(f,x)?;
                write!(f,")")?;
                Ok(())
            },
            Expression::CtorStruct(s,x,n) => {
                write!(f,"{} {{",s.name())?; // XXX module
                write_csl_named(f,x,n)?;
                write!(f,"}}")?;
                Ok(())
            },
            Expression::CtorEnum(e,b,v) => {
                write!(f,"{}:{} {:?}",e.name(),b,v)?; // XXX module
                Ok(())
            }
        }
    }
}

#[derive(PartialEq)]
pub struct Statement(pub Identifier,pub Vec<Expression>,pub String,pub u32);

impl fmt::Debug for Statement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f,"{}(",(self.0).name())?; // XXX module
        for (i,sub) in self.1.iter().enumerate() {
            if i > 0 {
                write!(f,",")?;
            }
            write!(f,"{:?}",sub)?;
        }
        write!(f,")")?;
        Ok(())
    }
}

#[derive(Debug,PartialEq)]
pub enum ParserStatement {
    Import(String),
    Use(String),
    Module(String),
    Inline(String,IdentifierPattern,InlineMode,f64),
    ExprMacro(IdentifierPattern),
    StmtMacro(IdentifierPattern),
    FuncDecl(IdentifierPattern,SignatureConstraint),
    ProcDecl(IdentifierPattern,SignatureConstraint),
    Regular(Statement),
    StructDef(IdentifierPattern,Vec<MemberType>,Vec<String>),
    EnumDef(IdentifierPattern,Vec<MemberType>,Vec<String>),
    EndOfParse
}

#[derive(Debug,PartialEq)]
pub struct ParseError {
    error: String
}

impl ParseError {
    pub fn new(error: &str, lexer: &Lexer) -> ParseError {
        let (file,line,col) = lexer.position();
        ParseError {
            error: format!("{} at line {} column {} in {}",error,line,col,file)
        }
    }

    pub fn message(&self) -> &str { &self.error }
}