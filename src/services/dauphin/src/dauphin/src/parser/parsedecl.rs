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
use super::node::{ ParserStatement, ParseError };
use crate::model::{ DefStore, IdentifierGuesser, IdentifierPattern };
use crate::typeinf::{ ArgumentExpressionConstraint, SignatureConstraint, SignatureMemberConstraint, MemberType };
use crate::typeinf::BaseType as BaseType2;
use super::lexutil::{ get_other, get_identifier };
use super::parseexpr::parse_full_identifier;

pub(in super) fn parse_exprdecl(lexer: &mut Lexer) -> Result<ParserStatement,ParseError> {
    lexer.get();
    let identifier = parse_full_identifier(lexer,None)?;
    Ok(ParserStatement::ExprMacro(identifier))
}

pub(in super) fn parse_stmtdecl(lexer: &mut Lexer) -> Result<ParserStatement,ParseError> {
    lexer.get();
    let identifier = parse_full_identifier(lexer,None)?;
    Ok(ParserStatement::StmtMacro(identifier))
}

pub(in super) fn parse_func(lexer: &mut Lexer, defstore: &DefStore, guesser: &mut IdentifierGuesser) -> Result<ParserStatement,ParseError> {
    lexer.get();
    let mut members = Vec::new();
    let identifier = parse_full_identifier(lexer,None)?;
    get_other(lexer,"(")?;
    loop {
        if lexer.peek(None,1)[0] == Token::Other(')') { lexer.get(); break; }
        members.push(SignatureMemberConstraint::RValue(parse_typesigexpr(lexer,defstore,guesser)?));
        match lexer.get() {
            Token::Other(',') => (),
            Token::Other(')') => break,
            _ => return Err(ParseError::new("Unexpected token (expected ) or ,)",lexer))
        };
    }
    if get_identifier(lexer)? != "becomes" {
        return Err(ParseError::new("missing 'becomes'",lexer));
    }
    members.insert(0,SignatureMemberConstraint::RValue(parse_typesigexpr(lexer,defstore,guesser)?));
    Ok(ParserStatement::FuncDecl(identifier,SignatureConstraint::new(&members)))
}

pub(in super) fn parse_proc(lexer: &mut Lexer, defstore: &DefStore, guesser: &mut IdentifierGuesser) -> Result<ParserStatement,ParseError> {
    lexer.get();
    let identifier = parse_full_identifier(lexer,None)?;
    let mut members = Vec::new();
    get_other(lexer,"(")?;
    loop {
        if lexer.peek(None,1)[0] == Token::Other(')') { break; }
        let member = parse_signature(lexer,defstore,guesser)?;
        members.push(member);
        match lexer.get() {
            Token::Other(',') => (),
            Token::Other(')') => break,
            _ => return Err(ParseError::new("Unexpected token (expected ) or ,)",lexer))
        };
    }
    Ok(ParserStatement::ProcDecl(identifier,SignatureConstraint::new(&members)))
}

pub fn parse_signature(lexer: &mut Lexer, defstore: &DefStore, guesser: &mut IdentifierGuesser) -> Result<SignatureMemberConstraint,ParseError> {
    let mut out = false;
    loop {
        match &lexer.peek(None,1)[0] {
            Token::Identifier(name) => {
                match &name[..] {
                    // XXX to go
                    "lvalue" => {
                        lexer.get();
                    },
                    "out" => {
                        out = true;
                        lexer.get();
                    },
                    _ => break
                }
            },
            _ => ()
        }
    }
    let argtype = parse_typesig(lexer,defstore,guesser)?;
    let member = if out {
        SignatureMemberConstraint::LValue(argtype)
    } else {
        SignatureMemberConstraint::RValue(argtype)
    };
    Ok(member)
}

fn id_to_type(pattern: &IdentifierPattern, lexer: &Lexer, defstore: &DefStore, guesser: &mut IdentifierGuesser) -> Result<BaseType2,ParseError> {
    let id = guesser.guess(lexer,&pattern).map_err(|e| ParseError::new(&e.to_string(),lexer))?;
    if defstore.get_struct_id(&id).is_ok() {
        Ok(BaseType2::StructType(id.clone()))
    } else if defstore.get_enum_id(&id).is_ok() {
        Ok(BaseType2::EnumType(id.clone()))
    } else {
        Err(ParseError::new(&format!("No such struct/enum '{}'",id),lexer))
    }
}

pub fn parse_type(lexer: &mut Lexer, defstore: &DefStore, guesser: &mut IdentifierGuesser) -> Result<MemberType,ParseError> {
    let pattern = parse_full_identifier(lexer,None)?;
    if pattern.0.is_none() {
        Ok(match &pattern.1[..] {
            "boolean" => MemberType::Base(BaseType2::BooleanType),
            "number" => MemberType::Base(BaseType2::NumberType),
            "string" => MemberType::Base(BaseType2::StringType),
            "bytes" => MemberType::Base(BaseType2::BytesType),
            "vec" => {
                get_other(lexer,"(")?;
                let out = MemberType::Vec(Box::new(parse_type(lexer,defstore,guesser)?));
                get_other(lexer,")")?;
                out
            },
            _ => MemberType::Base(id_to_type(&pattern,lexer,defstore,guesser)?)
        })
    } else {
        Ok(MemberType::Base(id_to_type(&pattern,lexer,defstore,guesser)?))
    }
}

pub fn parse_typesig(lexer: &mut Lexer, defstore: &DefStore, guesser: &mut IdentifierGuesser) -> Result<ArgumentExpressionConstraint,ParseError> {
    Ok(parse_typesigexpr(lexer,defstore,guesser)?)
}

pub fn parse_typesigexpr(lexer: &mut Lexer, defstore: &DefStore, guesser: &mut IdentifierGuesser) -> Result<ArgumentExpressionConstraint,ParseError> {
    let pattern = parse_full_identifier(lexer,None)?;
    if pattern.0.is_none() {
        Ok(match &pattern.1[..] {
            "boolean" => ArgumentExpressionConstraint::Base(BaseType2::BooleanType),
            "number" => ArgumentExpressionConstraint::Base(BaseType2::NumberType),
            "string" => ArgumentExpressionConstraint::Base(BaseType2::StringType),
            "bytes" => ArgumentExpressionConstraint::Base(BaseType2::BytesType),
            "vec" => {
                get_other(lexer,"(")?;
                let out =  ArgumentExpressionConstraint::Vec(Box::new(parse_typesigexpr(lexer,defstore,guesser)?));
                get_other(lexer,")")?;
                out
            },
            x => {
                if x.starts_with("_") {
                    ArgumentExpressionConstraint::Placeholder(x.to_string())
                } else {
                    ArgumentExpressionConstraint::Base(id_to_type(&pattern,lexer,defstore,guesser)?)
                }
            }
        })
    } else {
        Ok(ArgumentExpressionConstraint::Base(id_to_type(&pattern,lexer,defstore,guesser)?))
    }
}

fn parse_struct_short(lexer: &mut Lexer, defstore: &DefStore, guesser: &mut IdentifierGuesser) -> Result<(Vec<MemberType>,Vec<String>),ParseError> {
    let mut types = Vec::new();
    loop {
        let member_type = parse_type(lexer,defstore,guesser)?;
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

fn parse_struct_enum_full(lexer: &mut Lexer, defstore: &DefStore, guesser: &mut IdentifierGuesser) -> Result<(Vec<MemberType>,Vec<String>),ParseError> {
    let mut types = Vec::new();
    let mut names = Vec::new();
    if let Token::Other('}') = lexer.peek(None,1)[0] {
        lexer.get();
        return Ok((vec![],vec![]));
    }
    loop {
        names.push(get_identifier(lexer)?);
        get_other(lexer,":")?;
        let member_type = parse_type(lexer,defstore,guesser)?;
        types.push(member_type);
        match lexer.get() {
            Token::Other(',') => (),
            Token::Other('}') => break,
            _ => return Err(ParseError::new("Unexpected token (expected ; or ,)",lexer))
        }
    }
    Ok((types,names))
}

fn parse_struct_contents(lexer: &mut Lexer, defstore: &DefStore, guesser: &mut IdentifierGuesser) -> Result<(Vec<MemberType>,Vec<String>),ParseError> {
    let start = lexer.pos();
    Ok(match lexer.get() {
        Token::Identifier(_) => {
            let next = lexer.peek(None,1)[0].clone();
            lexer.back_to(start);
            match next {
                Token::Other(':') => parse_struct_enum_full(lexer,defstore,guesser)?,
                _ => parse_struct_short(lexer,defstore,guesser)?
            }
        },
        Token::Other('}') => {
            (vec![],vec![])
        },
        _ => return Err(ParseError::new("Unexpected token (expected ; or ,)",lexer))
    })
}

pub(in super) fn parse_struct(lexer: &mut Lexer, defstore: &DefStore, guesser: &mut IdentifierGuesser) -> Result<ParserStatement,ParseError> {
    lexer.get();
    let identifier = parse_full_identifier(lexer,None)?;
    get_other(lexer, "{")?;
    let (member_types,names) = parse_struct_contents(lexer,defstore,guesser)?;
    Ok(ParserStatement::StructDef(identifier,member_types,names))
}

pub(in super) fn parse_enum(lexer: &mut Lexer, defstore: &DefStore, guesser: &mut IdentifierGuesser) -> Result<ParserStatement,ParseError> {
    lexer.get();
    let identifier = parse_full_identifier(lexer,None)?;
    get_other(lexer, "{")?;
    let (member_types,names) = parse_struct_enum_full(lexer,defstore,guesser)?;
    Ok(ParserStatement::EnumDef(identifier,member_types,names))
}
