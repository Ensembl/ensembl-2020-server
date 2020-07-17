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

use crate::model::{ InlineMode, IdentifierPattern };
use crate::lexer::{ Lexer, LexerPosition };
use crate::typeinf::{ MemberType, SignatureConstraint };
use dauphin_interp::command::Identifier;

#[derive(PartialEq,Clone)]
pub enum Expression {
    Identifier(String),
    Number(String),
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

fn alpha_id(id: &str, args: &[Identifier], exprs: &[Expression]) -> Result<Expression,String> {
    if args.len() != exprs.len() {
        return Err(format!("{}: expected {} args, got {}",id,args.len(),exprs.len()));
    }
    for (i,arg) in args.iter().enumerate() {
        if arg.name() == id {
            return Ok(exprs[i].clone());
        }
    }
    Ok(Expression::Identifier(id.to_string()))
}

impl Expression {
    pub fn alpha(&self, args: &[Identifier], exprs: &[Expression]) -> Result<Expression,String> {
        Ok(match self {
            Expression::Identifier(s) => alpha_id(s,args,exprs)?,
            Expression::Operator(id,x) => Expression::Operator(id.clone(),x.iter().map(|x| x.alpha(args,exprs)).collect::<Result<_,String>>()?),
            Expression::Star(expr) => Expression::Star(Box::new(expr.alpha(args,exprs)?)),
            Expression::Square(expr) => Expression::Square(Box::new(expr.alpha(args,exprs)?)),
            Expression::Bracket(a,b) => Expression::Bracket(Box::new(a.alpha(args,exprs)?),Box::new(b.alpha(args,exprs)?)),
            Expression::Filter(a,b) => Expression::Filter(Box::new(a.alpha(args,exprs)?),Box::new(b.alpha(args,exprs)?)),
            Expression::Dot(a,s) => Expression::Dot(Box::new(a.alpha(args,exprs)?),s.to_string()),
            Expression::Query(a,s) => Expression::Query(Box::new(a.alpha(args,exprs)?),s.to_string()),
            Expression::Pling(a,s) => Expression::Pling(Box::new(a.alpha(args,exprs)?),s.to_string()),
            Expression::Vector(x) => Expression::Vector(x.iter().map(|x| x.alpha(args,exprs)).collect::<Result<_,String>>()?),
            Expression::CtorStruct(id,x,f) => Expression::CtorStruct(id.clone(),x.iter().map(|x| x.alpha(args,exprs)).collect::<Result<_,String>>()?,f.to_vec()),
            Expression::CtorEnum(id,f,a) => Expression::CtorEnum(id.clone(),f.to_string(),Box::new(a.alpha(args,exprs)?)),
            x => x.clone()
        })
    }
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

#[derive(PartialEq,Clone)]
pub struct Statement(pub Identifier,pub Vec<Expression>,pub LexerPosition);

impl Statement {
    pub fn alpha(&self, args: &[Identifier], exprs: &[Expression]) -> Result<Statement,String> {
        let out = self.1.iter().map(|x| x.alpha(args,exprs)).collect::<Result<_,String>>()?;
        Ok(Statement(self.0.clone(),out,self.2.clone()))
    }
}

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

#[derive(Debug,PartialEq,Clone)]
pub enum ParserStatement {
    Import(String),
    Use(String),
    Module(String),
    Inline(String,IdentifierPattern,InlineMode,f64),
    ExprMacro(IdentifierPattern,Vec<IdentifierPattern>,Expression),
    StmtMacro(IdentifierPattern,Vec<IdentifierPattern>,Vec<Statement>),
    FuncDecl(IdentifierPattern,SignatureConstraint),
    ProcDecl(IdentifierPattern,SignatureConstraint),
    Regular(Statement),
    StructDef(IdentifierPattern,Vec<MemberType>,Vec<String>),
    EnumDef(IdentifierPattern,Vec<MemberType>,Vec<String>),
    EndOfBlock,
    EndOfParse
}

#[derive(Debug,PartialEq)]
pub struct ParseError {
    error: String
}

impl ParseError {
    pub fn new(error: &str, lexer: &Lexer) -> ParseError {
        let pos = lexer.position();
        ParseError {
            error: format!("{} at {}",error,pos)
        }
    }

    pub fn message(&self) -> &str { &self.error }
}