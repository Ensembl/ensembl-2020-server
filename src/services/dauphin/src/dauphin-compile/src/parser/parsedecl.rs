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

use crate::lexer::{ Lexer, Token };
use super::node::{ ParserStatement, ParseError, Statement };
use crate::model::{ DefStore, IdentifierPattern };
use crate::typeinf::{ ArgumentExpressionConstraint, SignatureConstraint, SignatureMemberConstraint, MemberType };
use super::lexutil::{ get_other, get_identifier };
use super::parseexpr::{ parse_full_identifier, parse_expr };
use super::parsestmt::{ parse_statement };
use dauphin_interp::types::{ BaseType };

pub(in super) fn parse_exprdecl(lexer: &mut Lexer, defstore: &DefStore) -> Result<Vec<ParserStatement>,ParseError> {
    lexer.get();
    let identifier = parse_full_identifier(lexer,None)?;
    let args = parse_args(lexer)?;
    let expr = parse_expr(lexer,defstore,false)?;
    Ok(vec![ParserStatement::ExprMacro(identifier,args,expr)])
}

fn parse_args(lexer: &mut Lexer) -> Result<Vec<IdentifierPattern>,ParseError> {
    let mut out = vec![];
    get_other(lexer,"(")?;
    loop {
        if lexer.peek(None,1)[0] == Token::Other(')') { lexer.get(); break; }
        let id = parse_full_identifier(lexer,None)?;
        if id.0.is_some() {
            return Err(ParseError::new("module illegal in argument specifier",lexer));
        }
        out.push(id);
        match lexer.get() {
            Token::Other(',') => (),
            Token::Other(')') => break,
            _ => return Err(ParseError::new("Unexpected token (expected ) or ,)",lexer))
        };
    }
    Ok(out)
}

/* TODO dedup these from parser class */
fn ffwd_error(lexer: &mut Lexer) {
    loop {
        match lexer.get() {
            Token::Other(';') => return,
            Token::EndOfLex => return,
            _ => ()
        }
    }
}

fn parse_block_statement(lexer: &mut Lexer, defstore: &DefStore) -> Result<(Vec<Statement>,bool),ParseError> {
    let mut eof_seen = false;
    match parse_statement(lexer,defstore,true) {
        Ok(mut stmts) => {
            let mut out = vec![];
            for stmt in stmts.drain(..) {
                match stmt {
                    ParserStatement::Regular(r) => out.push(r),
                    ParserStatement::EndOfBlock => { eof_seen = true; },
                    _ => {
                        ffwd_error(lexer);
                        return Err(ParseError::new("Only regular statements allowed in macros",lexer));
                    }
                }            
            }
            Ok((out,eof_seen))
        },
        Err(s) => {
            ffwd_error(lexer);
            Err(s)
        }
    }
}

fn parse_block(lexer: &mut Lexer, defstore: &DefStore) -> Result<Vec<Statement>,ParseError> {
    let mut out = vec![];
    get_other(lexer,"{")?;
    loop {
        let (mut stmts,eof_seen) = parse_block_statement(lexer,defstore)?;
        out.append(&mut stmts);
        if eof_seen { break; }
    }
    get_other(lexer,"}")?;
    Ok(out)
}

pub(in super) fn parse_stmtdecl(lexer: &mut Lexer, defstore: &DefStore) -> Result<Vec<ParserStatement>,ParseError> {
    lexer.get();
    let identifier = parse_full_identifier(lexer,None)?;
    let args = parse_args(lexer)?;
    let block = parse_block(lexer,defstore)?;
    Ok(vec![ParserStatement::StmtMacro(identifier,args,block)])
}

pub(in super) fn parse_func(lexer: &mut Lexer, defstore: &DefStore) -> Result<Vec<ParserStatement>,ParseError> {
    lexer.get();
    let mut members = Vec::new();
    let identifier = parse_full_identifier(lexer,None)?;
    get_other(lexer,"(")?;
    loop {
        if lexer.peek(None,1)[0] == Token::Other(')') { lexer.get(); break; }
        members.push(SignatureMemberConstraint::RValue(parse_typesigexpr(lexer,defstore)?));
        match lexer.get() {
            Token::Other(',') => (),
            Token::Other(')') => break,
            _ => return Err(ParseError::new("Unexpected token (expected ) or ,)",lexer))
        };
    }
    if get_identifier(lexer)? != "becomes" {
        return Err(ParseError::new("missing 'becomes'",lexer));
    }
    members.insert(0,SignatureMemberConstraint::RValue(parse_typesigexpr(lexer,defstore)?));
    Ok(vec![ParserStatement::FuncDecl(identifier,SignatureConstraint::new(&members))])
}

pub(in super) fn parse_proc(lexer: &mut Lexer, defstore: &DefStore) -> Result<Vec<ParserStatement>,ParseError> {
    lexer.get();
    let identifier = parse_full_identifier(lexer,None)?;
    let mut members = Vec::new();
    get_other(lexer,"(")?;
    loop {
        if lexer.peek(None,1)[0] == Token::Other(')') { lexer.get(); break; }
        let member = parse_signature(lexer,defstore)?;
        members.push(member);
        match lexer.get() {
            Token::Other(',') => (),
            Token::Other(')') => break,
            _ => return Err(ParseError::new("Unexpected token (expected ) or ,)",lexer))
        };
    }
    Ok(vec![ParserStatement::ProcDecl(identifier,SignatureConstraint::new(&members))])
}

pub fn parse_signature(lexer: &mut Lexer, defstore: &DefStore) -> Result<SignatureMemberConstraint,ParseError> {
    let mut out = false;
    let mut stomp = false;
    loop {
        match &lexer.peek(None,1)[0] {
            Token::Identifier(name) => {
                match &name[..] {
                    "mask" => {
                        out = true;
                        lexer.get();
                    },
                    "stomp" => {
                        stomp = true;
                        lexer.get();
                    },
                    _ => break
                }
            },
            _ => ()
        }
    }
    let argtype = parse_typesig(lexer,defstore)?;
    let member = if out {
        SignatureMemberConstraint::LValue(argtype,stomp)
    } else {
        SignatureMemberConstraint::RValue(argtype)
    };
    Ok(member)
}

fn id_to_type(pattern: &IdentifierPattern, lexer: &Lexer, defstore: &DefStore) -> Result<BaseType,ParseError> {
    let id = defstore.pattern_to_identifier(lexer,&pattern,true).map_err(|e| ParseError::new(&e.to_string(),lexer))?;
    if defstore.get_struct_id(&id.0).is_ok() {
        Ok(BaseType::StructType(id.0.clone()))
    } else if defstore.get_enum_id(&id.0).is_ok() {
        Ok(BaseType::EnumType(id.0.clone()))
    } else {
        Err(ParseError::new(&format!("No such struct/enum '{}'",id.0),lexer))
    }
}

pub fn parse_type(lexer: &mut Lexer, defstore: &DefStore) -> Result<MemberType,ParseError> {
    let pattern = parse_full_identifier(lexer,None)?;
    if pattern.0.is_none() {
        Ok(match &pattern.1[..] {
            "boolean" => MemberType::Base(BaseType::BooleanType),
            "number" => MemberType::Base(BaseType::NumberType),
            "string" => MemberType::Base(BaseType::StringType),
            "bytes" => MemberType::Base(BaseType::BytesType),
            "vec" => {
                get_other(lexer,"(")?;
                let out = MemberType::Vec(Box::new(parse_type(lexer,defstore)?));
                get_other(lexer,")")?;
                out
            },
            _ => MemberType::Base(id_to_type(&pattern,lexer,defstore)?)
        })
    } else {
        Ok(MemberType::Base(id_to_type(&pattern,lexer,defstore)?))
    }
}

pub fn parse_typesig(lexer: &mut Lexer, defstore: &DefStore) -> Result<ArgumentExpressionConstraint,ParseError> {
    Ok(parse_typesigexpr(lexer,defstore)?)
}

pub fn parse_typesigexpr(lexer: &mut Lexer, defstore: &DefStore) -> Result<ArgumentExpressionConstraint,ParseError> {
    let pattern = parse_full_identifier(lexer,None)?;
    if pattern.0.is_none() {
        Ok(match &pattern.1[..] {
            "boolean" => ArgumentExpressionConstraint::Base(BaseType::BooleanType),
            "number" => ArgumentExpressionConstraint::Base(BaseType::NumberType),
            "string" => ArgumentExpressionConstraint::Base(BaseType::StringType),
            "bytes" => ArgumentExpressionConstraint::Base(BaseType::BytesType),
            "vec" => {
                get_other(lexer,"(")?;
                let out =  ArgumentExpressionConstraint::Vec(Box::new(parse_typesigexpr(lexer,defstore)?));
                get_other(lexer,")")?;
                out
            },
            x => {
                if x.starts_with("_") {
                    ArgumentExpressionConstraint::Placeholder(x.to_string())
                } else {
                    ArgumentExpressionConstraint::Base(id_to_type(&pattern,lexer,defstore)?)
                }
            }
        })
    } else {
        Ok(ArgumentExpressionConstraint::Base(id_to_type(&pattern,lexer,defstore)?))
    }
}

fn parse_struct_short(lexer: &mut Lexer, defstore: &DefStore) -> Result<(Vec<MemberType>,Vec<String>),ParseError> {
    let mut types = Vec::new();
    loop {
        let member_type = parse_type(lexer,defstore)?;
        types.push(member_type);
        match lexer.get() {
            Token::Other(',') => (),
            Token::Other('}') => break,
            _ => return Err(ParseError::new("Unexpected token (expected ; or ,)",lexer))
        };
    }
    let len = types.len();
    Ok((types,(0..len).into_iter().map(|x| (x.to_string())).collect()))
}

fn parse_struct_enum_full(lexer: &mut Lexer, defstore: &DefStore) -> Result<(Vec<MemberType>,Vec<String>),ParseError> {
    let mut types = Vec::new();
    let mut names = Vec::new();
    if let Token::Other('}') = lexer.peek(None,1)[0] {
        lexer.get();
        return Ok((vec![],vec![]));
    }
    loop {
        names.push(get_identifier(lexer)?);
        get_other(lexer,":")?;
        let member_type = parse_type(lexer,defstore)?;
        types.push(member_type);
        match lexer.get() {
            Token::Other(',') => (),
            Token::Other('}') => break,
            _ => return Err(ParseError::new("Unexpected token (expected ; or ,)",lexer))
        }
    }
    Ok((types,names))
}

fn parse_struct_contents(lexer: &mut Lexer, defstore: &DefStore) -> Result<(Vec<MemberType>,Vec<String>),ParseError> {
    let start = lexer.pos();
    Ok(match lexer.get() {
        Token::Identifier(_) => {
            let next = lexer.peek(None,1)[0].clone();
            lexer.back_to(start);
            match next {
                Token::Other(':') => parse_struct_enum_full(lexer,defstore)?,
                _ => parse_struct_short(lexer,defstore)?
            }
        },
        Token::Other('}') => {
            (vec![],vec![])
        },
        _ => return Err(ParseError::new("Unexpected token (expected ; or ,)",lexer))
    })
}

pub(in super) fn parse_struct(lexer: &mut Lexer, defstore: &DefStore) -> Result<Vec<ParserStatement>,ParseError> {
    lexer.get();
    let identifier = parse_full_identifier(lexer,None)?;
    get_other(lexer, "{")?;
    let (member_types,names) = parse_struct_contents(lexer,defstore)?;
    Ok(vec![ParserStatement::StructDef(identifier,member_types,names)])
}

pub(in super) fn parse_enum(lexer: &mut Lexer, defstore: &DefStore) -> Result<Vec<ParserStatement>,ParseError> {
    lexer.get();
    let identifier = parse_full_identifier(lexer,None)?;
    get_other(lexer, "{")?;
    let (member_types,names) = parse_struct_enum_full(lexer,defstore)?;
    Ok(vec![ParserStatement::EnumDef(identifier,member_types,names)])
}
